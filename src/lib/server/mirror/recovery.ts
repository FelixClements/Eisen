import type { BackupRef } from '$lib/sync/types';
import type { MirrorDatabasePort, RecoveryObjectPort } from './types';
import { MirrorNotFoundError } from './types';

export function createRecoveryStoreApi(
	database: MirrorDatabasePort,
	recoveryObjects: RecoveryObjectPort
) {
	return {
		async storeRecoveryPackage(accountId: string, packageId: string, ciphertext: string) {
			const r2Key = `backups/${accountId}/${packageId}`;
			await recoveryObjects.put(r2Key, ciphertext);
			await database.insertBackupMeta(accountId, {
				packageId,
				r2Key,
				createdAt: Date.now()
			});
		},
		async listRecoveryPackages(accountId: string): Promise<BackupRef[]> {
			const rows = await database.listBackupMeta(accountId);
			return rows.map((r) => ({ id: r.packageId, createdAt: r.createdAt }));
		},
		async getRecoveryPackage(accountId: string, packageId: string): Promise<string> {
			const meta = await database.getBackupMeta(accountId, packageId);
			if (!meta) throw new MirrorNotFoundError('Recovery package not found.');
			const body = await recoveryObjects.get(meta.r2Key);
			if (body === null) throw new MirrorNotFoundError('Recovery package blob missing.');
			return body;
		}
	};
}
