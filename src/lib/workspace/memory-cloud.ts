import { VaultParamsExistError, EisenErrorException } from '$lib/workspace/types';
import type { EncryptedMirror } from '$lib/server/encrypted-mirror';
import type { BackupRef } from '$lib/workspace/types';
import type { CloudPort, SyncPushBatch, VaultParams } from '$lib/workspace/ports';

export function cloudPortFor(mirror: EncryptedMirror, accountId: string): CloudPort {
	return {
		async getVaultParams() {
			return mirror.getVaultParams(accountId);
		},
		async createVaultParams(params: VaultParams) {
			try {
				await mirror.createVaultParams(accountId, params);
			} catch (e) {
				if (e instanceof VaultParamsExistError) {
					throw new EisenErrorException({ code: 'vault-exists' });
				}
				throw e;
			}
		},
		async sync(batch: SyncPushBatch) {
			return mirror.exchangeSync(accountId, batch);
		},
		async putBackup(pkg) {
			await mirror.storeRecoveryPackage(accountId, pkg.id, pkg.text);
			return { id: pkg.id, createdAt: Date.now() };
		},
		async listBackups(): Promise<BackupRef[]> {
			return mirror.listRecoveryPackages(accountId);
		},
		async getBackup(packageId) {
			return mirror.getRecoveryPackage(accountId, packageId);
		},
		async scheduleWake(w) {
			await mirror.scheduleWake(accountId, w);
		},
		async registerPush(sub) {
			await mirror.registerPushSubscription(accountId, sub);
		}
	};
}