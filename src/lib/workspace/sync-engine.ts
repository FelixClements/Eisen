import { EisenErrorException, type EisenError } from './types';
import { remoteWins } from './merge';
import type { SyncPort } from './ports';
import type { CatalogTask, TaskCodec } from './task-codec';
import type { TaskRepository } from './task-repository';

export type SyncPhase = 'idle' | 'syncing' | 'offline' | 'error';

export type SyncEngine = {
	run(): Promise<void>;
	get phase(): SyncPhase;
	get lastError(): EisenError | null;
};

export function createSyncEngine(opts: {
	cloud: SyncPort;
	codec: TaskCodec;
	repo: TaskRepository;
	accountId: string;
	getDeviceId: () => string;
	getLastVersion: () => number;
	setLastVersion: (version: number) => void;
	getTasks: () => Iterable<CatalogTask>;
	findTask: (id: string) => CatalogTask | undefined;
	ensurePersisted: (task: CatalogTask) => Promise<void>;
	applyRemote: (task: CatalogTask, syncVersion: number) => Promise<void>;
	onNotify: () => void;
	scheduleWake: () => Promise<void>;
}): SyncEngine {
	let phase: SyncPhase = 'idle';
	let lastError: EisenError | null = null;
	let syncing: Promise<void> | null = null;

	async function runSync(): Promise<void> {
		if (syncing) return syncing;
		syncing = (async () => {
			phase = 'syncing';
			lastError = null;
			opts.onNotify();
			try {
				const changes = await Promise.all(
					[...opts.getTasks()]
						.filter((t) => t.dirty)
						.map(async (t) => {
							await opts.ensurePersisted(t);
							return {
								recordId: t.id,
								encryptedBlob: t.blob,
								modifiedAt: t.updatedAt,
								deviceId: t.deviceId,
								deleted: t.deleted ? 1 : 0
							};
						})
				);
				const result = await opts.cloud.sync({ lastVersion: opts.getLastVersion(), changes });
				for (const record of result.changes) {
					const syncVersion = record.syncVersion ?? 0;
					if (syncVersion === 0) continue;
					const payload = await opts.codec.decryptPayload(record.encryptedBlob);
					if (!payload) continue;
					const existing = opts.findTask(record.recordId);
					const incoming = { modifiedAt: record.modifiedAt, deviceId: payload.deviceId };
					if (
						existing &&
						!remoteWins(
							{ modifiedAt: existing.updatedAt, deviceId: existing.deviceId },
							incoming
						) &&
						existing.dirty
					) {
						continue;
					}
					const next = opts.codec.fromSyncRecord(record, payload);
					await opts.applyRemote(next, syncVersion);
				}
				for (const t of opts.getTasks()) {
					if (t.dirty && changes.some((c) => c.recordId === t.id && c.modifiedAt === t.updatedAt)) {
						t.dirty = false;
						t.syncState = 'synced';
						await opts.repo.updateDirty(t.id, 0);
					}
				}
				opts.setLastVersion(result.lastVersion);
				await opts.repo.putMeta({
					accountId: opts.accountId,
					deviceId: opts.getDeviceId(),
					lastVersion: result.lastVersion
				});
				phase = 'idle';
				await opts.scheduleWake();
			} catch (e) {
				if (e instanceof EisenErrorException) {
					lastError = e.error;
					phase = e.error.code === 'offline' ? 'offline' : 'error';
				} else {
					lastError = { code: 'server', status: 0 };
					phase = 'error';
				}
			} finally {
				syncing = null;
				opts.onNotify();
			}
			if (phase === 'idle' && [...opts.getTasks()].some((t) => t.dirty)) {
				await runSync();
			}
		})();
		return syncing;
	}

	return {
		run: runSync,
		get phase() {
			return phase;
		},
		get lastError() {
			return lastError;
		}
	};
}
