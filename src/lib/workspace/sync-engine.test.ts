import { describe, expect, it } from 'vitest';
import { createSyncEngine } from './sync-engine';
import type { CatalogTask, TaskCodec } from './task-codec';
import type { EncryptedTaskPayload } from './types';
import type { SyncPort, SyncPullBatch } from './ports';
import type { TaskRepository } from './task-repository';

function catalogTask(partial: Partial<CatalogTask> & Pick<CatalogTask, 'id'>): CatalogTask {
	return {
		title: 'Task',
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
		createdAt: 1,
		updatedAt: 100,
		syncState: 'local',
		deviceId: 'device-a',
		deleted: false,
		dirty: true,
		blob: 'blob-local',
		...partial
	};
}

function payload(deviceId: string): EncryptedTaskPayload {
	return {
		title: 'Task',
		notes: '',
		tag: '',
		important: true,
		urgent: true,
		dueAt: null,
		remindAt: null,
		pinned: false,
		completed: false,
		archived: false,
		createdAt: 1,
		deviceId
	};
}

function stubCodec(opts?: { failBlob?: string }): TaskCodec {
	return {
		toPayload: () => payload('device-a'),
		fromPayload: (p, row) => catalogTask({ id: row.id, updatedAt: row.updatedAt, deviceId: p.deviceId }),
		encryptPayload: async () => 'blob',
		decryptPayload: async (blob) => (blob === opts?.failBlob ? null : payload('from-payload')),
		encryptTask: async () => 'blob',
		toStoredRow: (task, accountId) => ({
			id: task.id,
			accountId,
			updatedAt: task.updatedAt,
			deleted: task.deleted ? 1 : 0,
			blob: task.blob,
			dirty: task.dirty ? 1 : 0,
			syncVersion: 0
		}),
		toView: (task) => ({
			id: task.id,
			title: task.title,
			notes: task.notes,
			tag: task.tag,
			important: task.important,
			urgent: task.urgent,
			quadrant: task.quadrant,
			dueAt: task.dueAt,
			remindAt: task.remindAt,
			pinned: task.pinned,
			completed: task.completed,
			archived: task.archived,
			createdAt: task.createdAt,
			updatedAt: task.updatedAt,
			syncState: task.syncState
		}),
		fromRecoveryRow: () => catalogTask({ id: 'x' }),
		fromSyncRecord: (record, p) =>
			catalogTask({
				id: record.recordId,
				updatedAt: record.modifiedAt,
				deviceId: p.deviceId,
				blob: record.encryptedBlob,
				dirty: false,
				syncState: 'synced'
			})
	};
}

function stubRepo(): TaskRepository {
	return {
		loadRows: async () => [],
		put: async () => {},
		updateDirty: async () => {},
		getMeta: async () => undefined,
		putMeta: async () => {},
		ensureMeta: async () => ({ accountId: 'acct', deviceId: 'device-a', lastVersion: 0 }),
		close: () => {}
	};
}

async function runEngine(opts: {
	tasks: CatalogTask[];
	pull: SyncPullBatch;
	failBlob?: string;
}) {
	const applied: string[] = [];
	const cloud: SyncPort = {
		sync: async () => opts.pull
	};
	const engine = createSyncEngine({
		cloud,
		codec: stubCodec({ failBlob: opts.failBlob }),
		repo: stubRepo(),
		accountId: 'acct',
		getDeviceId: () => 'device-a',
		getLastVersion: () => 0,
		setLastVersion: () => {},
		getTasks: () => opts.tasks,
		findTask: (id) => opts.tasks.find((t) => t.id === id),
		ensurePersisted: async () => {},
		applyRemote: async (task) => {
			applied.push(task.id);
		},
		onNotify: () => {},
		scheduleWake: async () => {}
	});
	await engine.run();
	return { applied, tasks: opts.tasks };
}

describe('createSyncEngine', () => {
	it('clears dirty when the pull echoes the pushed envelope', async () => {
		const local = catalogTask({ id: 'r1', updatedAt: 100, deviceId: 'device-a', dirty: true });
		const { tasks } = await runEngine({
			tasks: [local],
			pull: {
				lastVersion: 1,
				changes: [
					{
						recordId: 'r1',
						encryptedBlob: 'blob-local',
						modifiedAt: 100,
						deviceId: 'device-a',
						syncVersion: 1,
						deleted: 0
					}
				]
			}
		});
		expect(tasks[0].dirty).toBe(false);
	});

	it('keeps dirty when the server omits the pushed record', async () => {
		const local = catalogTask({ id: 'r1', updatedAt: 100, deviceId: 'device-a', dirty: true });
		const { tasks, applied } = await runEngine({
			tasks: [local],
			pull: { lastVersion: 5, changes: [] }
		});
		expect(tasks[0].dirty).toBe(true);
		expect(applied).toEqual([]);
	});

	it('does not apply an older remote over a non-dirty local row', async () => {
		const local = catalogTask({
			id: 'r1',
			updatedAt: 200,
			deviceId: 'device-a',
			dirty: false,
			syncState: 'synced'
		});
		const { applied } = await runEngine({
			tasks: [local],
			pull: {
				lastVersion: 2,
				changes: [
					{
						recordId: 'r1',
						encryptedBlob: 'blob-old',
						modifiedAt: 100,
						deviceId: 'device-z',
						syncVersion: 2,
						deleted: 0
					}
				]
			}
		});
		expect(applied).toEqual([]);
	});

	it('does not apply or clear dirty when decrypt fails', async () => {
		const local = catalogTask({
			id: 'r1',
			updatedAt: 100,
			deviceId: 'device-a',
			dirty: true,
			blob: 'bad-blob'
		});
		const { applied, tasks } = await runEngine({
			tasks: [local],
			failBlob: 'bad-blob',
			pull: {
				lastVersion: 1,
				changes: [
					{
						recordId: 'r1',
						encryptedBlob: 'bad-blob',
						modifiedAt: 100,
						deviceId: 'device-a',
						syncVersion: 1,
						deleted: 0
					}
				]
			}
		});
		expect(applied).toEqual([]);
		expect(tasks[0].dirty).toBe(true);
	});
});
