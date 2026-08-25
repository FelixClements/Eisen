import { describe, expect, it } from 'vitest';
import { createEncryptedMirror } from './encrypted-mirror';
import {
	memoryMirrorDatabase,
	memoryRecoveryObjects,
	recordingPushDispatch
} from './adapters/memory';

const ACCOUNT = 'acct-1';

function newMirror() {
	return createEncryptedMirror({
		database: memoryMirrorDatabase(),
		recoveryObjects: memoryRecoveryObjects(),
		pushDispatch: recordingPushDispatch()
	});
}

describe('EncryptedMirror exchangeSync LWW', () => {
	it('rejects an older push without bumping sync version', async () => {
		const mirror = newMirror();
		await mirror.exchangeSync(ACCOUNT, {
			lastVersion: 0,
			changes: [
				{
					recordId: 'r1',
					encryptedBlob: 'blob-new',
					modifiedAt: 200,
					deviceId: 'device-b',
					deleted: 0
				}
			]
		});
		const afterWinner = await mirror.exchangeSync(ACCOUNT, {
			lastVersion: 0,
			changes: [
				{
					recordId: 'r1',
					encryptedBlob: 'blob-old',
					modifiedAt: 100,
					deviceId: 'device-a',
					deleted: 0
				}
			]
		});
		const stored = afterWinner.changes.find((c) => c.recordId === 'r1');
		expect(stored?.encryptedBlob).toBe('blob-new');
		expect(stored?.modifiedAt).toBe(200);
	});

	it('accepts a newer push and returns it on pull', async () => {
		const mirror = newMirror();
		await mirror.exchangeSync(ACCOUNT, {
			lastVersion: 0,
			changes: [
				{
					recordId: 'r1',
					encryptedBlob: 'blob-v1',
					modifiedAt: 100,
					deviceId: 'device-a',
					deleted: 0
				}
			]
		});
		const updated = await mirror.exchangeSync(ACCOUNT, {
			lastVersion: 0,
			changes: [
				{
					recordId: 'r1',
					encryptedBlob: 'blob-v2',
					modifiedAt: 300,
					deviceId: 'device-b',
					deleted: 0
				}
			]
		});
		const stored = updated.changes.find((c) => c.recordId === 'r1');
		expect(stored?.encryptedBlob).toBe('blob-v2');
	});

	it('breaks equal modifiedAt ties with deviceId', async () => {
		const mirror = newMirror();
		await mirror.exchangeSync(ACCOUNT, {
			lastVersion: 0,
			changes: [
				{
					recordId: 'r1',
					encryptedBlob: 'blob-low',
					modifiedAt: 100,
					deviceId: 'aaa',
					deleted: 0
				}
			]
		});
		await mirror.exchangeSync(ACCOUNT, {
			lastVersion: 0,
			changes: [
				{
					recordId: 'r1',
					encryptedBlob: 'blob-high',
					modifiedAt: 100,
					deviceId: 'bbb',
					deleted: 0
				}
			]
		});
		const pull = await mirror.exchangeSync(ACCOUNT, { lastVersion: 0, changes: [] });
		const stored = pull.changes.find((c) => c.recordId === 'r1');
		expect(stored?.deviceId).toBe('bbb');
		expect(stored?.encryptedBlob).toBe('blob-high');
	});
});
