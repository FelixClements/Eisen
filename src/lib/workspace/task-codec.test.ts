import 'fake-indexeddb/auto';
import { describe, expect, it } from 'vitest';
import { deriveVaultKey, newKdfSalt } from '$lib/crypto';
import { createTaskCodec, type CatalogTask } from './task-codec';
import { createTaskRepository } from './task-repository';

const PASSWORD = 'correct horse';
const ITER = 1;

async function testVaultKey() {
	const salt = await newKdfSalt();
	return deriveVaultKey(PASSWORD, salt, ITER);
}

function sampleTask(overrides: Partial<CatalogTask> = {}): CatalogTask {
	return {
		id: 'task-1',
		title: 'Buy milk',
		notes: '',
		tag: '',
		important: true,
		urgent: true,
		quadrant: 'do-now',
		dueAt: null,
		remindAt: null,
		pinned: false,
		completed: false,
		archived: false,
		createdAt: 1000,
		updatedAt: 1000,
		syncState: 'local',
		deviceId: 'dev-a',
		deleted: false,
		dirty: true,
		blob: '',
		...overrides
	};
}

describe('TaskCodec', () => {
	it('round-trips encrypt and decrypt', async () => {
		const key = await testVaultKey();
		const codec = createTaskCodec(key);
		const task = sampleTask();
		const blob = await codec.encryptTask(task);
		const payload = await codec.decryptPayload(blob);
		expect(payload).toMatchObject({
			title: 'Buy milk',
			important: true,
			urgent: true,
			deviceId: 'dev-a'
		});
	});

	it('fromPayload uses row updatedAt, deleted, and dirty', async () => {
		const key = await testVaultKey();
		const codec = createTaskCodec(key);
		const payload = codec.toPayload(sampleTask());
		const task = codec.fromPayload(payload, {
			id: 'task-1',
			updatedAt: 2000,
			deleted: true,
			dirty: false
		});
		expect(task.updatedAt).toBe(2000);
		expect(task.deleted).toBe(true);
		expect(task.dirty).toBe(false);
		expect(task.syncState).toBe('synced');
	});

	it('returns null for undecryptable blobs', async () => {
		const key = await testVaultKey();
		const codec = createTaskCodec(key);
		expect(await codec.decryptPayload('not-valid-ciphertext')).toBeNull();
	});

	it('toView maps dirty to local syncState', async () => {
		const key = await testVaultKey();
		const codec = createTaskCodec(key);
		expect(codec.toView(sampleTask({ dirty: true })).syncState).toBe('local');
		expect(codec.toView(sampleTask({ dirty: false })).syncState).toBe('synced');
	});
});

describe('TaskRepository', () => {
	it('bootstraps meta with deviceId and lastVersion', async () => {
		const repo = createTaskRepository({ dbName: 'repo-meta' });
		const meta = await repo.ensureMeta('acct-1', () => 'device-uuid-1');
		expect(meta.deviceId).toBe('device-uuid-1');
		expect(meta.lastVersion).toBe(0);
		const again = await repo.ensureMeta('acct-1', () => 'other');
		expect(again.deviceId).toBe('device-uuid-1');
		repo.close();
	});

	it('persists dirty flag on tasks', async () => {
		const repo = createTaskRepository({ dbName: 'repo-dirty' });
		await repo.put({
			id: 't1',
			accountId: 'acct-1',
			updatedAt: 100,
			deleted: 0,
			blob: 'blob',
			dirty: 1,
			syncVersion: 0
		});
		const rows = await repo.loadRows('acct-1');
		expect(rows[0]?.dirty).toBe(1);
		await repo.updateDirty('t1', 0);
		const rowsAfter = await repo.loadRows('acct-1');
		expect(rowsAfter[0]?.dirty).toBe(0);
		repo.close();
	});
});
