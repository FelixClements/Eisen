import { EISEN_DB_NAME, EisenWebDB } from './db';

export type WrappedKeyStore = {
	put(accountId: string, key: CryptoKey): Promise<void>;
	get(accountId: string): Promise<CryptoKey | null>;
	delete(accountId: string): Promise<void>;
};

export function createWrappedKeyStore(opts?: {
	dbName?: string;
	indexedDB?: IDBFactory;
	IDBKeyRange?: typeof globalThis.IDBKeyRange;
}): WrappedKeyStore {
	const dbName = opts?.dbName ?? EISEN_DB_NAME;

	function openDb() {
		return new EisenWebDB(dbName, opts?.indexedDB, opts?.IDBKeyRange);
	}

	return {
		async put(accountId, key) {
			const db = openDb();
			await db.wrappedKeys.put({ accountId, key });
			db.close();
		},
		async get(accountId) {
			const db = openDb();
			const row = await db.wrappedKeys.get(accountId);
			db.close();
			return row?.key ?? null;
		},
		async delete(accountId) {
			const db = openDb();
			await db.wrappedKeys.delete(accountId);
			db.close();
		}
	};
}
