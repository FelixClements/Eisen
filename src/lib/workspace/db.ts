import Dexie, { type Table } from 'dexie';

export const EISEN_DB_NAME = 'eisen-web-v2';

export type StoredTask = {
	id: string;
	accountId: string;
	updatedAt: number;
	deleted: number;
	blob: string;
	dirty: number;
	syncVersion: number;
};

export type StoredMeta = {
	accountId: string;
	deviceId: string;
	lastVersion: number;
};

export type WrappedKey = {
	accountId: string;
	key: CryptoKey;
};

export class EisenWebDB extends Dexie {
	tasks!: Table<StoredTask, string>;
	meta!: Table<StoredMeta, string>;
	wrappedKeys!: Table<WrappedKey, string>;

	constructor(name: string, indexedDB?: IDBFactory, IDBKeyRange?: typeof globalThis.IDBKeyRange) {
		super(name, indexedDB && IDBKeyRange ? { indexedDB, IDBKeyRange } : undefined);
		this.version(2).stores({
			tasks: 'id, accountId, updatedAt, deleted, dirty, syncVersion',
			meta: 'accountId',
			wrappedKeys: 'accountId'
		});
	}
}
