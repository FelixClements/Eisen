import { deriveAuthVerifier } from '$lib/crypto';
import { authClient } from '$lib/auth-client';
import { signInTypedOrVerifier } from './account-sign-in';
import { openWorkspace, type Workspace, type WorkspaceAdapters } from './workspace';
import { vaultKeyFromPassword } from './vault-key';
import { EisenErrorException } from './types';
import type { CloudPort, RemindersPort } from './ports';
import { createWrappedKeyStore, type WrappedKeyStore } from './wrapped-key-store';

export type VaultSession = {
	unlock(opts: {
		mode: 'sign-in' | 'sign-up';
		email: string;
		password: string;
		name?: string;
	}): Promise<{ accountId: string }>;
	resume(accountId: string): Promise<CryptoKey | null>;
	lock(accountId: string): Promise<void>;
	openWorkspace(
		accountId: string,
		key: CryptoKey,
		adapters: Omit<WorkspaceAdapters, 'onSignOut' | 'cloud'> & {
			onSignOut?: () => Promise<void>;
		}
	): Workspace;
};

export type VaultSessionDeps = {
	cloud: CloudPort;
	keyStore?: WrappedKeyStore;
	kdfIterations?: number;
	dbName?: string;
	indexedDB?: IDBFactory;
	IDBKeyRange?: typeof globalThis.IDBKeyRange;
};

export function createVaultSession(deps: VaultSessionDeps): VaultSession {
	const cloud = deps.cloud;
	const keyStore =
		deps.keyStore ??
		createWrappedKeyStore({
			dbName: deps.dbName,
			indexedDB: deps.indexedDB,
			IDBKeyRange: deps.IDBKeyRange
		});
	const kdfIterations = deps.kdfIterations;

	return {
		async unlock(opts) {
			if (opts.password.length < 8) {
				throw new EisenErrorException({
					code: 'weak-passphrase',
					reason: 'Password must be at least 8 characters.'
				});
			}
			const verifier = await deriveAuthVerifier(opts.password, opts.email);
			const existing = await authClient.getSession();
			const alreadySignedIn = opts.mode === 'sign-in' && Boolean(existing.data?.user);
			if (!alreadySignedIn) {
				if (opts.mode === 'sign-up') {
					const { error } = await authClient.signUp.email({
						email: opts.email,
						password: verifier,
						name: opts.name || opts.email.split('@')[0]
					});
					if (error) throw new Error(error.message ?? 'Sign up failed');
				} else {
					await signInTypedOrVerifier({
						typedPassword: opts.password,
						verifier,
						signIn: async (password) => authClient.signIn.email({ email: opts.email, password }),
						upgrade: async (currentPassword, newPassword) => {
							await authClient.changePassword({ currentPassword, newPassword });
						}
					});
				}
			}
			const session = await authClient.getSession();
			const user = session.data?.user;
			if (!user) throw new Error('Signed in but no session.');
			const key = await vaultKeyFromPassword({ password: opts.password, cloud, iterations: kdfIterations });
			await keyStore.put(user.id, key);
			return { accountId: user.id };
		},

		async resume(accountId) {
			return keyStore.get(accountId);
		},

		async lock(accountId) {
			await keyStore.delete(accountId);
		},

		openWorkspace(accountId, key, adapters) {
			return openWorkspace({
				account: { id: accountId },
				vaultKey: key,
				adapters: {
					cloud,
					reminders: adapters.reminders,
					clock: adapters.clock,
					dbName: adapters.dbName ?? deps.dbName,
					indexedDB: adapters.indexedDB ?? deps.indexedDB,
					IDBKeyRange: adapters.IDBKeyRange ?? deps.IDBKeyRange,
					kdfIterations: adapters.kdfIterations ?? kdfIterations,
					onSignOut: adapters.onSignOut
				}
			});
		}
	};
}
