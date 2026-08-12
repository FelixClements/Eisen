import { db, type Todo } from './db';
import { encrypt, decrypt } from './crypto';

const OWNER_ID_KEY = 'eisen-owner-id';
const LAST_SYNC_KEY = 'eisen-last-version';

export function getOwnerId(): string {
	let id = localStorage.getItem(OWNER_ID_KEY);
	if (!id) {
		id = crypto.randomUUID();
		localStorage.setItem(OWNER_ID_KEY, id);
	}
	return id;
}

export function getLastSyncVersion(): number {
	const raw = localStorage.getItem(LAST_SYNC_KEY);
	return raw ? parseInt(raw, 10) : 0;
}

export function setLastSyncVersion(version: number): void {
	localStorage.setItem(LAST_SYNC_KEY, String(version));
}

export type SyncRecord = {
	recordId: string;
	encryptedBlob: string;
	modifiedAt: number;
	syncVersion?: number;
	deleted: number;
};

export async function sync(masterKey: CryptoKey, fetch = globalThis.fetch): Promise<void> {
	const ownerId = getOwnerId();
	const lastVersion = getLastSyncVersion();
	const todos = await db.todos.toArray();

	const changes: SyncRecord[] = await Promise.all(
		todos.map(async (todo) => {
			const payload = JSON.stringify({
				title: todo.title,
				notes: todo.notes,
				completed: todo.completed,
				quadrant: todo.quadrant
			});
			return {
				recordId: todo.id,
				encryptedBlob: await encrypt(payload, masterKey),
				modifiedAt: todo.local_updated_at,
				deleted: todo.deleted
			};
		})
	);

	const response = await fetch('/api/sync', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ ownerId, lastVersion, changes })
	});

	if (!response.ok) {
		throw new Error(`Sync failed: ${response.status}`);
	}

	const { changes: serverRecords, lastVersion: newVersion } = (await response.json()) as {
		changes: SyncRecord[];
		lastVersion: number;
	};

	for (const record of serverRecords) {
		const syncVersion = record.syncVersion ?? 0;
		if (syncVersion === 0) continue;

		const plaintext = await decrypt(record.encryptedBlob, masterKey);
		const content = JSON.parse(plaintext) as {
			title: string;
			notes: string;
			completed: boolean;
			quadrant: Todo['quadrant'];
		};
		const existing = await db.todos.get(record.recordId);

		if (!existing || (existing.sync_version ?? 0) < syncVersion) {
			await db.todos.put({
				id: record.recordId,
				...content,
				local_updated_at: record.modifiedAt,
				sync_version: syncVersion,
				deleted: record.deleted,
				encrypted_blob: record.encryptedBlob
			});
		}
	}

	setLastSyncVersion(newVersion);
}
