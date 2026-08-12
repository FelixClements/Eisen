import { db, type Task } from './db';
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

type EncryptedTask = Omit<
	Task,
	'id' | 'updatedAt' | 'sync_version' | 'deleted' | 'encrypted_blob'
>;

export async function sync(masterKey: CryptoKey, fetch = globalThis.fetch): Promise<void> {
	const ownerId = getOwnerId();
	const lastVersion = getLastSyncVersion();
	const tasks = await db.tasks.toArray();

	const changes: SyncRecord[] = await Promise.all(
		tasks.map(async (task) => {
			const payload: EncryptedTask = {
				title: task.title,
				description: task.description,
				isImportant: task.isImportant,
				isUrgent: task.isUrgent,
				dueDate: task.dueDate,
				reminderAt: task.reminderAt,
				isCompleted: task.isCompleted,
				isArchived: task.isArchived,
				isPinned: task.isPinned,
				category: task.category,
				createdAt: task.createdAt
			};
			return {
				recordId: task.id,
				encryptedBlob: await encrypt(JSON.stringify(payload), masterKey),
				modifiedAt: task.updatedAt,
				deleted: task.deleted
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
		const content = JSON.parse(plaintext) as EncryptedTask;
		const existing = await db.tasks.get(record.recordId);

		if (!existing || (existing.sync_version ?? 0) < syncVersion) {
			await db.tasks.put({
				id: record.recordId,
				...content,
				updatedAt: record.modifiedAt,
				sync_version: syncVersion,
				deleted: record.deleted,
				encrypted_blob: record.encryptedBlob
			});
		}
	}

	setLastSyncVersion(newVersion);
}
