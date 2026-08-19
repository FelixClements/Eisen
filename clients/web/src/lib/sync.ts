import { db, getDeviceId, type Task } from './db';
import { encrypt, decrypt } from './crypto';
import { scheduleNextWake } from './notifications';

const LAST_SYNC_PREFIX = 'eisen-last-version-';

export type SyncRecord = {
	recordId: string;
	encryptedBlob: string;
	modifiedAt: number;
	syncVersion?: number;
	deleted: number;
};

type EncryptedTask = Omit<Task, 'id' | 'userId' | 'updatedAt' | 'sync_version' | 'deleted' | 'encrypted_blob'>;

function getLastSyncVersion(userId: string): number {
	const raw = localStorage.getItem(`${LAST_SYNC_PREFIX}${userId}`);
	return raw ? parseInt(raw, 10) : 0;
}

function setLastSyncVersion(userId: string, version: number): void {
	localStorage.setItem(`${LAST_SYNC_PREFIX}${userId}`, String(version));
}

export async function sync(userId: string, masterKey: CryptoKey, fetchFn = globalThis.fetch): Promise<void> {
	const deviceId = await getDeviceId(userId);
	const lastVersion = getLastSyncVersion(userId);
	const tasks = await db.tasks.where('userId').equals(userId).toArray();

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

	const response = await fetchFn('/api/sync', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ deviceId, lastVersion, changes })
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
				userId,
				...content,
				updatedAt: record.modifiedAt,
				sync_version: syncVersion,
				deleted: record.deleted,
				encrypted_blob: record.encryptedBlob
			});
		}
	}

	setLastSyncVersion(userId, newVersion);
	await scheduleNextWake(userId);
}
