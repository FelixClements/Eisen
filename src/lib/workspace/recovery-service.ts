import type { BackupRef, Outcome } from './types';
import type { BackupPort, Clock } from './ports';
import type { CatalogTask, TaskCodec } from './task-codec';

export type RecoveryService = {
	exportRecoveryPackage(recoveryPassphrase: string): Promise<Outcome<Blob>>;
	backupToCloud(recoveryPassphrase: string): Promise<Outcome<BackupRef>>;
	listCloudBackups(): Promise<Outcome<BackupRef[]>>;
	restoreFromCloud(packageId: string, recoveryPassphrase: string): Promise<Outcome<void>>;
	importRecoveryPackage(file: File, recoveryPassphrase: string): Promise<Outcome<void>>;
};

export function createRecoveryService(opts: {
	accountId: string;
	codec: TaskCodec;
	cloud: BackupPort;
	clock: Clock;
	kdfIterations: number;
	getDeviceId: () => string;
	getTasks: () => Iterable<CatalogTask>;
	persist: (task: CatalogTask) => Promise<void>;
	setTask: (task: CatalogTask) => void;
	onImported: () => void;
	triggerSync: () => void;
}): RecoveryService {
	async function packageTasks(recoveryPassphrase: string): Promise<Outcome<string>> {
		if (recoveryPassphrase.length < 8) {
			return {
				ok: false,
				error: { code: 'weak-passphrase', reason: 'Recovery passphrase must be at least 8 characters.' }
			};
		}
		try {
			const { deriveVaultKey, encrypt, newKdfSalt, toBase64 } = await import('$lib/crypto');
			const salt = await newKdfSalt();
			const wrapKey = await deriveVaultKey(recoveryPassphrase, salt, opts.kdfIterations);
			const body = JSON.stringify({
				accountId: opts.accountId,
				tasks: [...opts.getTasks()].map((t) => ({
					id: t.id,
					updatedAt: t.updatedAt,
					deleted: t.deleted,
					payload: opts.codec.toPayload(t)
				}))
			});
			const ciphertext = await encrypt(body, wrapKey);
			return {
				ok: true,
				value: JSON.stringify({
					version: 1,
					accountId: opts.accountId,
					kdfSalt: toBase64(salt),
					ciphertext
				})
			};
		} catch (e) {
			return { ok: false, error: { code: 'storage', message: e instanceof Error ? e.message : 'export failed' } };
		}
	}

	async function applyPackageText(text: string, recoveryPassphrase: string): Promise<Outcome<void>> {
		try {
			const { deriveVaultKey, fromBase64, decrypt: dec } = await import('$lib/crypto');
			const pkg = JSON.parse(text) as {
				version: number;
				accountId: string;
				kdfSalt: string;
				ciphertext: string;
			};
			if (pkg.version !== 1) return { ok: false, error: { code: 'package-corrupt' } };
			if (pkg.accountId !== opts.accountId) return { ok: false, error: { code: 'package-wrong-account' } };
			const wrapKey = await deriveVaultKey(recoveryPassphrase, fromBase64(pkg.kdfSalt), opts.kdfIterations);
			let plain: string;
			try {
				plain = await dec(pkg.ciphertext, wrapKey);
			} catch {
				return { ok: false, error: { code: 'wrong-passphrase' } };
			}
			const body = JSON.parse(plain) as {
				tasks: Array<{
					id: string;
					updatedAt: number;
					deleted: boolean;
					payload: import('./types').EncryptedTaskPayload;
				}>;
			};
			for (const row of body.tasks) {
				const task = opts.codec.fromRecoveryRow(row);
				opts.setTask(task);
				await opts.persist(task);
			}
			opts.onImported();
			opts.triggerSync();
			return { ok: true, value: undefined };
		} catch {
			return { ok: false, error: { code: 'package-corrupt' } };
		}
	}

	return {
		async exportRecoveryPackage(recoveryPassphrase) {
			const packed = await packageTasks(recoveryPassphrase);
			if (!packed.ok) return packed;
			return { ok: true, value: new Blob([packed.value], { type: 'application/eisen-recovery' }) };
		},

		async backupToCloud(recoveryPassphrase) {
			const packed = await packageTasks(recoveryPassphrase);
			if (!packed.ok) return packed;
			try {
				const id = opts.clock.uuid();
				const ref = await opts.cloud.putBackup({
					id,
					text: packed.value,
					deviceId: opts.getDeviceId()
				});
				return { ok: true, value: ref };
			} catch {
				return { ok: false, error: { code: 'server', status: 0 } };
			}
		},

		async listCloudBackups() {
			try {
				return { ok: true, value: await opts.cloud.listBackups() };
			} catch {
				return { ok: false, error: { code: 'server', status: 0 } };
			}
		},

		async restoreFromCloud(packageId, recoveryPassphrase) {
			try {
				const text = await opts.cloud.getBackup(packageId);
				return applyPackageText(text, recoveryPassphrase);
			} catch {
				return { ok: false, error: { code: 'server', status: 0 } };
			}
		},

		async importRecoveryPackage(file, recoveryPassphrase) {
			return applyPackageText(await file.text(), recoveryPassphrase);
		}
	};
}
