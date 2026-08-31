import type { SyncPullBatch, SyncPushBatch, SyncRecord } from '$lib/sync/types';
import type { MirrorDatabasePort } from './types';

export function createRecordSyncApi(database: MirrorDatabasePort) {
	return {
		async exchangeSync(accountId: string, batch: SyncPushBatch): Promise<SyncPullBatch> {
			for (const change of batch.changes) {
				await database.applyLwwUpsert(accountId, {
					recordId: change.recordId,
					encryptedBlob: change.encryptedBlob,
					modifiedAt: change.modifiedAt,
					deviceId: change.deviceId,
					deleted: change.deleted
				});
			}
			const rows = await database.recordsAfter(accountId, batch.lastVersion ?? 0);
			const lastVersion = await database.maxSyncVersion(accountId);
			const changes: SyncRecord[] = rows.map((row) => ({
				recordId: row.recordId,
				encryptedBlob: row.encryptedBlob,
				modifiedAt: row.modifiedAt,
				deviceId: row.deviceId,
				syncVersion: row.syncVersion,
				deleted: row.deleted
			}));
			return { changes, lastVersion };
		}
	};
}
