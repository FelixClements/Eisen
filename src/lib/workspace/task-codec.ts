import { decrypt, encrypt } from '$lib/crypto';
import { quadrantOf } from './matrix';
import type { EncryptedTaskPayload, Quadrant, TaskView } from './types';
import type { StoredTask } from './db';

export type CatalogTask = {
	id: string;
	title: string;
	notes: string;
	tag: string;
	important: boolean;
	urgent: boolean;
	quadrant: Quadrant;
	dueAt: number | null;
	remindAt: number | null;
	pinned: boolean;
	completed: boolean;
	archived: boolean;
	createdAt: number;
	updatedAt: number;
	syncState: 'local' | 'synced';
	deviceId: string;
	deleted: boolean;
	dirty: boolean;
	blob: string;
};

export type TaskRowMeta = {
	id: string;
	updatedAt: number;
	deleted: boolean;
	dirty: boolean;
	syncVersion?: number;
};

export type TaskCodec = {
	toPayload(task: CatalogTask): EncryptedTaskPayload;
	fromPayload(payload: EncryptedTaskPayload, row: TaskRowMeta): CatalogTask;
	encryptPayload(payload: EncryptedTaskPayload): Promise<string>;
	decryptPayload(blob: string): Promise<EncryptedTaskPayload | null>;
	encryptTask(task: CatalogTask): Promise<string>;
	toStoredRow(task: CatalogTask, accountId: string, syncVersion?: number): StoredTask;
	toView(task: CatalogTask): TaskView;
	fromRecoveryRow(row: {
		id: string;
		updatedAt: number;
		deleted: boolean;
		payload: EncryptedTaskPayload;
	}): CatalogTask;
	fromSyncRecord(record: {
		recordId: string;
		encryptedBlob: string;
		modifiedAt: number;
		deleted: number;
	}, payload: EncryptedTaskPayload): CatalogTask;
};

export function createTaskCodec(vaultKey: CryptoKey): TaskCodec {
	return {
		toPayload(task) {
			return {
				title: task.title,
				notes: task.notes,
				tag: task.tag,
				important: task.important,
				urgent: task.urgent,
				dueAt: task.dueAt,
				remindAt: task.remindAt,
				pinned: task.pinned,
				completed: task.completed,
				archived: task.archived,
				createdAt: task.createdAt,
				deviceId: task.deviceId
			};
		},

		fromPayload(payload, row) {
			return {
				id: row.id,
				title: payload.title,
				notes: payload.notes,
				tag: payload.tag,
				important: payload.important,
				urgent: payload.urgent,
				quadrant: quadrantOf(payload),
				dueAt: payload.dueAt,
				remindAt: payload.remindAt,
				pinned: payload.pinned,
				completed: payload.completed,
				archived: payload.archived,
				createdAt: payload.createdAt,
				updatedAt: row.updatedAt,
				syncState: row.dirty ? 'local' : 'synced',
				deviceId: payload.deviceId,
				deleted: row.deleted,
				dirty: row.dirty,
				blob: ''
			};
		},

		async encryptPayload(payload) {
			return encrypt(JSON.stringify(payload), vaultKey);
		},

		async decryptPayload(blob) {
			try {
				return JSON.parse(await decrypt(blob, vaultKey)) as EncryptedTaskPayload;
			} catch {
				return null;
			}
		},

		async encryptTask(task) {
			return encrypt(JSON.stringify(this.toPayload(task)), vaultKey);
		},

		toStoredRow(task, accountId, syncVersion = 0) {
			return {
				id: task.id,
				accountId,
				updatedAt: task.updatedAt,
				deleted: task.deleted ? 1 : 0,
				blob: task.blob,
				dirty: task.dirty ? 1 : 0,
				syncVersion
			};
		},

		toView(task) {
			return {
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
				syncState: task.dirty ? 'local' : 'synced'
			};
		},

		fromRecoveryRow(row) {
			return {
				...this.fromPayload(row.payload, {
					id: row.id,
					updatedAt: row.updatedAt,
					deleted: row.deleted,
					dirty: true
				}),
				syncState: 'local',
				dirty: true,
				blob: ''
			};
		},

		fromSyncRecord(record, payload) {
			return {
				...this.fromPayload(payload, {
					id: record.recordId,
					updatedAt: record.modifiedAt,
					deleted: record.deleted === 1,
					dirty: false
				}),
				syncState: 'synced',
				dirty: false,
				blob: record.encryptedBlob
			};
		}
	};
}
