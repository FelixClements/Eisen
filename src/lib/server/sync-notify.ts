import type { SyncPullBatch, SyncPushBatch } from '$lib/sync/types';

export type VersionedSync = {
	maxSyncVersion(accountId: string): Promise<number>;
	exchangeSync(accountId: string, batch: SyncPushBatch): Promise<SyncPullBatch>;
};

export type FanoutNotify = {
	notify(accountId: string, version: number): Promise<void>;
};

export async function exchangeAndNotify(
	mirror: VersionedSync,
	fanout: FanoutNotify | null,
	accountId: string,
	batch: SyncPushBatch
): Promise<SyncPullBatch> {
	const before = await mirror.maxSyncVersion(accountId);
	const result = await mirror.exchangeSync(accountId, batch);
	const after = await mirror.maxSyncVersion(accountId);
	if (fanout && after > before) {
		try {
			await fanout.notify(accountId, after);
		} catch {
			// The blobs are already stored. A missed ping is repaired when the tab becomes visible.
		}
	}
	return result;
}
