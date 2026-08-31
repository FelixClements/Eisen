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

	it('keeps the same recordId isolated per Account', async () => {
		const db = memoryMirrorDatabase();
		const objects = memoryRecoveryObjects();
		const mirror = createEncryptedMirror({
			database: db,
			recoveryObjects: objects,
			pushDispatch: recordingPushDispatch()
		});
		await mirror.exchangeSync('acct-a', {
			lastVersion: 0,
			changes: [
				{
					recordId: 'shared',
					encryptedBlob: 'blob-a',
					modifiedAt: 100,
					deviceId: 'device-a',
					deleted: 0
				}
			]
		});
		await mirror.exchangeSync('acct-b', {
			lastVersion: 0,
			changes: [
				{
					recordId: 'shared',
					encryptedBlob: 'blob-b',
					modifiedAt: 100,
					deviceId: 'device-b',
					deleted: 0
				}
			]
		});
		const pullA = await mirror.exchangeSync('acct-a', { lastVersion: 0, changes: [] });
		const pullB = await mirror.exchangeSync('acct-b', { lastVersion: 0, changes: [] });
		expect(pullA.changes.find((c) => c.recordId === 'shared')?.encryptedBlob).toBe('blob-a');
		expect(pullB.changes.find((c) => c.recordId === 'shared')?.encryptedBlob).toBe('blob-b');
	});

	it('assigns distinct sync versions to sequential accepts', async () => {
		const mirror = newMirror();
		const first = await mirror.exchangeSync(ACCOUNT, {
			lastVersion: 0,
			changes: [
				{
					recordId: 'r1',
					encryptedBlob: 'blob-1',
					modifiedAt: 100,
					deviceId: 'device-a',
					deleted: 0
				}
			]
		});
		const second = await mirror.exchangeSync(ACCOUNT, {
			lastVersion: first.lastVersion,
			changes: [
				{
					recordId: 'r2',
					encryptedBlob: 'blob-2',
					modifiedAt: 200,
					deviceId: 'device-a',
					deleted: 0
				}
			]
		});
		const v1 = first.changes.find((c) => c.recordId === 'r1')?.syncVersion;
		const v2 = second.changes.find((c) => c.recordId === 'r2')?.syncVersion;
		expect(v1).toBe(1);
		expect(v2).toBe(2);
		expect(second.lastVersion).toBe(2);
	});

	it('keeps the same recovery packageId isolated per Account', async () => {
		const db = memoryMirrorDatabase();
		const objects = memoryRecoveryObjects();
		const mirror = createEncryptedMirror({
			database: db,
			recoveryObjects: objects,
			pushDispatch: recordingPushDispatch()
		});
		await mirror.storeRecoveryPackage('acct-a', 'pkg-shared', 'cipher-a');
		await mirror.storeRecoveryPackage('acct-b', 'pkg-shared', 'cipher-b');
		expect(await mirror.getRecoveryPackage('acct-a', 'pkg-shared')).toBe('cipher-a');
		expect(await mirror.getRecoveryPackage('acct-b', 'pkg-shared')).toBe('cipher-b');
	});
});
