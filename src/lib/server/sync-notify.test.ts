import { describe, expect, it, vi } from 'vitest';
import { createEncryptedMirror } from './encrypted-mirror';
import { memoryMirrorDatabase, memoryRecoveryObjects, recordingPushDispatch } from './adapters/memory';
import { exchangeAndNotify } from './sync-notify';
import type { SyncPushBatch } from '$lib/sync/types';

const ACCOUNT = 'acct-1';

function mirror() {
	return createEncryptedMirror({
		database: memoryMirrorDatabase(),
		recoveryObjects: memoryRecoveryObjects(),
		pushDispatch: recordingPushDispatch()
	});
}

function push(modifiedAt: number, blob: string): SyncPushBatch {
	return {
		lastVersion: 0,
		changes: [
			{
				recordId: 'r1',
				encryptedBlob: blob,
				modifiedAt,
				deviceId: 'device-b',
				deleted: 0
			}
		]
	};
}

describe('exchangeAndNotify', () => {
	it('notifies with the version a write advances to', async () => {
		const store = mirror();
		const notify = vi.fn(async () => {});
		const result = await exchangeAndNotify(store, { notify }, ACCOUNT, push(200, 'blob-new'));
		expect(result.lastVersion).toBeGreaterThan(0);
		expect(notify).toHaveBeenCalledWith(ACCOUNT, result.lastVersion);
		expect(result.changes[0]?.encryptedBlob).toBe('blob-new');
	});

	it('does not notify on an empty pull', async () => {
		const store = mirror();
		const notify = vi.fn(async () => {});
		await exchangeAndNotify(store, { notify }, ACCOUNT, { lastVersion: 0, changes: [] });
		expect(notify).not.toHaveBeenCalled();
	});

	it('does not notify when last-write-wins rejects the push', async () => {
		const store = mirror();
		await store.exchangeSync(ACCOUNT, push(200, 'blob-new'));
		const notify = vi.fn(async () => {});
		await exchangeAndNotify(store, { notify }, ACCOUNT, push(100, 'blob-old'));
		expect(notify).not.toHaveBeenCalled();
	});

	it('returns the exchange when notify throws', async () => {
		const store = mirror();
		const result = await exchangeAndNotify(
			store,
			{
				notify: async () => {
					throw new Error('fanout down');
				}
			},
			ACCOUNT,
			push(200, 'blob-new')
		);
		expect(result.changes[0]?.encryptedBlob).toBe('blob-new');
		expect(result.lastVersion).toBeGreaterThan(0);
	});

	it('returns the exchange when fanout is missing', async () => {
		const store = mirror();
		const result = await exchangeAndNotify(store, null, ACCOUNT, push(200, 'blob-new'));
		expect(result.lastVersion).toBeGreaterThan(0);
	});
});
