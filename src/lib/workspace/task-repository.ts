import { EISEN_DB_NAME, EisenWebDB, type StoredMeta, type StoredTask } from './db';

export type TaskRepository = {
	loadRows(accountId: string): Promise<StoredTask[]>;
	put(row: StoredTask): Promise<void>;
	updateDirty(id: string, dirty: 0 | 1): Promise<void>;
	getMeta(accountId: string): Promise<StoredMeta | undefined>;
	putMeta(meta: StoredMeta): Promise<void>;
	ensureMeta(accountId: string, newDeviceId: () => string): Promise<StoredMeta>;
	close(): void;
};

export function createTaskRepository(opts: {
	dbName?: string;
	indexedDB?: IDBFactory;
	IDBKeyRange?: typeof globalThis.IDBKeyRange;
}): TaskRepository {
	const db = new EisenWebDB(opts.dbName ?? EISEN_DB_NAME, opts.indexedDB, opts.IDBKeyRange);

	return {
		async loadRows(accountId) {
			return db.tasks.where('accountId').equals(accountId).toArray();
		},

		async put(row) {
			await db.tasks.put(row);
		},

		async updateDirty(id, dirty) {
			await db.tasks.update(id, { dirty });
		},

		async getMeta(accountId) {
			return db.meta.get(accountId);
		},

		async putMeta(meta) {
			await db.meta.put(meta);
		},

		async ensureMeta(accountId, newDeviceId) {
			let meta = await db.meta.get(accountId);
			if (!meta) {
				meta = { accountId, deviceId: newDeviceId(), lastVersion: 0 };
				await db.meta.put(meta);
			}
			return meta;
		},

		close() {
			db.close();
		}
	};
}
