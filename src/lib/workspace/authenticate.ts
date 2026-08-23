import { deriveAuthVerifier } from '$lib/crypto';
import { authClient } from '$lib/auth-client';
import { httpCloud } from './http-cloud';
import { vaultKeyFromPassword, wrapVaultKey } from './vault-key';
import { EisenErrorException } from './types';

export async function authenticateWithVault(opts: {
	mode: 'sign-in' | 'sign-up';
	email: string;
	password: string;
	name?: string;
}): Promise<{ id: string }> {
	if (opts.password.length < 8) {
		throw new EisenErrorException({
			code: 'weak-passphrase',
			reason: 'Password must be at least 8 characters.'
		});
	}
	const verifier = await deriveAuthVerifier(opts.password, opts.email);
	const existing = await authClient.getSession();
	if (opts.mode === 'sign-in' && existing.data?.user) {
		// already have a session; still need the password to derive the Vault key
	} else if (opts.mode === 'sign-up') {
		const { error } = await authClient.signUp.email({
			email: opts.email,
			password: verifier,
			name: opts.name || opts.email.split('@')[0]
		});
		if (error) throw new Error(error.message ?? 'Sign up failed');
	} else {
		const { error } = await authClient.signIn.email({
			email: opts.email,
			password: verifier
		});
		if (error) throw new Error(error.message ?? 'Sign in failed');
	}
	const session = await authClient.getSession();
	const user = session.data?.user;
	if (!user) throw new Error('Signed in but no session.');
	const key = await vaultKeyFromPassword({ password: opts.password, cloud: httpCloud() });
	await wrapVaultKey({ accountId: user.id, key });
	return { id: user.id };
}
