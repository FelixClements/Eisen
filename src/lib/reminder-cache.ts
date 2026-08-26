const DB_NAME = 'eisen-reminder-cache';
const DB_VERSION = 1;
const STORE = 'reminders';

export type CachedReminder = {
	id: string;
	title: string;
	remindAt: number;
};

function openDb(): Promise<IDBDatabase> {
	return new Promise((resolve, reject) => {
		const req = indexedDB.open(DB_NAME, DB_VERSION);
		req.onerror = () => reject(req.error);
		req.onsuccess = () => resolve(req.result);
		req.onupgradeneeded = () => {
			const db = req.result;
			if (!db.objectStoreNames.contains(STORE)) {
				db.createObjectStore(STORE, { keyPath: 'id' });
			}
		};
	});
}

export async function syncReminderCache(reminders: CachedReminder[]): Promise<void> {
	const db = await openDb();
	const tx = db.transaction(STORE, 'readwrite');
	const store = tx.objectStore(STORE);
	await new Promise<void>((resolve, reject) => {
		const clear = store.clear();
		clear.onerror = () => reject(clear.error);
		clear.onsuccess = () => resolve();
	});
	for (const row of reminders) {
		await new Promise<void>((resolve, reject) => {
			const put = store.put(row);
			put.onerror = () => reject(put.error);
			put.onsuccess = () => resolve();
		});
	}
	await new Promise<void>((resolve, reject) => {
		tx.oncomplete = () => resolve();
		tx.onerror = () => reject(tx.error);
	});
	db.close();
}

export async function clearReminderCache(): Promise<void> {
	const db = await openDb();
	const tx = db.transaction(STORE, 'readwrite');
	await new Promise<void>((resolve, reject) => {
		const clear = tx.objectStore(STORE).clear();
		clear.onerror = () => reject(clear.error);
		clear.onsuccess = () => resolve();
	});
	await new Promise<void>((resolve, reject) => {
		tx.oncomplete = () => resolve();
		tx.onerror = () => reject(tx.error);
	});
	db.close();
}

export async function getDueRemindersFromCache(now: number): Promise<CachedReminder[]> {
	const db = await openDb();
	const tx = db.transaction(STORE, 'readonly');
	const store = tx.objectStore(STORE);
	const rows = await new Promise<CachedReminder[]>((resolve, reject) => {
		const req = store.getAll();
		req.onerror = () => reject(req.error);
		req.onsuccess = () => resolve((req.result as CachedReminder[]) ?? []);
	});
	await new Promise<void>((resolve, reject) => {
		tx.oncomplete = () => resolve();
		tx.onerror = () => reject(tx.error);
	});
	db.close();
	return rows.filter((r) => r.remindAt <= now);
}
