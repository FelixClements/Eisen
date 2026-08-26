import type { BackupRef, SyncRecord } from '$lib/workspace/types';
export { VaultParamsExistError } from '$lib/workspace/types';
import { VaultParamsExistError } from '$lib/workspace/types';
import type { SyncPullBatch, SyncPushBatch, VaultParams } from '$lib/workspace/ports';
import { applyRecordLww } from '$lib/sync/record-lww';

export type PushSubscriptionRow = {
	deviceId: string;
	endpoint: string;
	p256dh: string;
	auth: string;
};

export type WakeScheduleRow = {
	deviceId: string;
	wakeAt: number;
	nonce: string;
};

export type VaultRecordRow = {
	recordId: string;
	encryptedBlob: string;
	modifiedAt: number;
	deviceId: string;
	syncVersion: number;
	deleted: number;
};

export type BackupMetaRow = {
	packageId: string;
	r2Key: string;
	createdAt: number;
};

export type DueWake = {
	id: string;
	userId: string;
	deviceId: string;
	endpoint: string;
	p256dh: string;
	auth: string;
};

export interface MirrorDatabasePort {
	getVaultParams(accountId: string): Promise<VaultParams | null>;
	insertVaultParams(accountId: string, params: VaultParams): Promise<'ok' | 'exists'>;
	nextSyncVersion(accountId: string): Promise<number>;
	getRecord(accountId: string, recordId: string): Promise<VaultRecordRow | null>;
	upsertRecord(accountId: string, record: VaultRecordRow): Promise<void>;
	recordsAfter(accountId: string, lastVersion: number): Promise<VaultRecordRow[]>;
	maxSyncVersion(accountId: string): Promise<number>;
	insertBackupMeta(accountId: string, row: BackupMetaRow): Promise<void>;
	listBackupMeta(accountId: string): Promise<BackupMetaRow[]>;
	getBackupMeta(accountId: string, packageId: string): Promise<BackupMetaRow | null>;
	upsertPushSubscription(accountId: string, sub: PushSubscriptionRow): Promise<void>;
	deletePushSubscription(endpoint: string): Promise<void>;
	insertWake(accountId: string, id: string, schedule: WakeScheduleRow): Promise<void>;
	dueWakes(now: number): Promise<DueWake[]>;
	markWakeSent(id: string): Promise<void>;
	incrementWakeAttempts(id: string): Promise<number>;
}

export interface RecoveryObjectPort {
	put(key: string, body: string): Promise<void>;
	get(key: string): Promise<string | null>;
}

export interface PushDispatchPort {
	send(sub: PushSubscriptionRow, payload: string): Promise<void>;
}

export type EncryptedMirrorPorts = {
	database: MirrorDatabasePort;
	recoveryObjects: RecoveryObjectPort;
	pushDispatch: PushDispatchPort;
};

export type EncryptedMirror = {
	getVaultParams(accountId: string): Promise<VaultParams | null>;
	createVaultParams(accountId: string, params: VaultParams): Promise<void>;
	exchangeSync(accountId: string, batch: SyncPushBatch): Promise<SyncPullBatch>;
	storeRecoveryPackage(accountId: string, packageId: string, ciphertext: string): Promise<void>;
	listRecoveryPackages(accountId: string): Promise<BackupRef[]>;
	getRecoveryPackage(accountId: string, packageId: string): Promise<string>;
	registerPushSubscription(accountId: string, sub: PushSubscriptionRow): Promise<void>;
	scheduleWake(accountId: string, schedule: WakeScheduleRow): Promise<{ scheduleId: string }>;
	dispatchDueWakes(now: number): Promise<{ sent: number; failed: number }>;
};
export class MirrorNotFoundError extends Error {
	constructor(message: string) {
		super(message);
	}
}

export class PushSubscriptionConflictError extends Error {
	constructor() {
		super('Push subscription already registered to another account.');
	}
}

export function createEncryptedMirror(ports: EncryptedMirrorPorts): EncryptedMirror {
	return {
		async getVaultParams(accountId) {
			return ports.database.getVaultParams(accountId);
		},

		async createVaultParams(accountId, params) {
			const result = await ports.database.insertVaultParams(accountId, params);
			if (result === 'exists') throw new VaultParamsExistError();
		},

		async exchangeSync(accountId, batch) {
			for (const change of batch.changes) {
				const existing = await ports.database.getRecord(accountId, change.recordId);
				const decision = applyRecordLww(
					existing ? { modifiedAt: existing.modifiedAt, deviceId: existing.deviceId } : null,
					{ modifiedAt: change.modifiedAt, deviceId: change.deviceId }
				);
				if (decision === 'reject') continue;
				const nextVersion = await ports.database.nextSyncVersion(accountId);
				await ports.database.upsertRecord(accountId, {
					recordId: change.recordId,
					encryptedBlob: change.encryptedBlob,
					modifiedAt: change.modifiedAt,
					deviceId: change.deviceId,
					syncVersion: nextVersion,
					deleted: change.deleted
				});
			}
			const rows = await ports.database.recordsAfter(accountId, batch.lastVersion ?? 0);
			const lastVersion = await ports.database.maxSyncVersion(accountId);
			const changes: SyncRecord[] = rows.map((row) => ({
				recordId: row.recordId,
				encryptedBlob: row.encryptedBlob,
				modifiedAt: row.modifiedAt,
				deviceId: row.deviceId,
				syncVersion: row.syncVersion,
				deleted: row.deleted
			}));
			return { changes, lastVersion };
		},

		async storeRecoveryPackage(accountId, packageId, ciphertext) {
			const r2Key = `backups/${accountId}/${packageId}`;
			await ports.recoveryObjects.put(r2Key, ciphertext);
			await ports.database.insertBackupMeta(accountId, {
				packageId,
				r2Key,
				createdAt: Date.now()
			});
		},

		async listRecoveryPackages(accountId) {
			const rows = await ports.database.listBackupMeta(accountId);
			return rows.map((r) => ({ id: r.packageId, createdAt: r.createdAt }));
		},

		async getRecoveryPackage(accountId, packageId) {
			const meta = await ports.database.getBackupMeta(accountId, packageId);
			if (!meta) throw new MirrorNotFoundError('Recovery package not found.');
			const body = await ports.recoveryObjects.get(meta.r2Key);
			if (body === null) throw new MirrorNotFoundError('Recovery package blob missing.');
			return body;
		},

		async registerPushSubscription(accountId, sub) {
			await ports.database.upsertPushSubscription(accountId, sub);
		},

		async scheduleWake(accountId, schedule) {
			const scheduleId = crypto.randomUUID();
			await ports.database.insertWake(accountId, scheduleId, schedule);
			return { scheduleId };
		},

		async dispatchDueWakes(now) {
			const due = await ports.database.dueWakes(now);
			let sent = 0;
			let failed = 0;
			for (const row of due) {
				try {
					await ports.pushDispatch.send(
						{
							deviceId: row.deviceId,
							endpoint: row.endpoint,
							p256dh: row.p256dh,
							auth: row.auth
						},
						JSON.stringify({ type: 'wake' })
					);
					await ports.database.markWakeSent(row.id);
					sent++;
				} catch (err) {
					failed++;
					const status = err && typeof err === 'object' && 'status' in err ? Number(err.status) : 0;
					const host = (() => {
						try {
							return new URL(row.endpoint).host;
						} catch {
							return 'unknown';
						}
					})();
					console.error(`push dispatch failed host=${host} status=${status || 'unknown'}`);
					if (status === 404 || status === 410) {
						await ports.database.deletePushSubscription(row.endpoint);
						await ports.database.markWakeSent(row.id);
						continue;
					}
					const attempts = await ports.database.incrementWakeAttempts(row.id);
					if (attempts >= 5) {
						await ports.database.markWakeSent(row.id);
					}
				}
			}
			return { sent, failed };
		}
	};
}
