import type { BackupRef, SyncPullBatch, SyncPushBatch, SyncRecord, VaultParams } from '$lib/sync/types';

export type { BackupRef, SyncRecord, VaultParams, SyncPullBatch, SyncPushBatch };

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
	applyLwwUpsert(
		accountId: string,
		incoming: Omit<VaultRecordRow, 'syncVersion'>
	): Promise<'accept' | 'reject'>;
	insertBackupMeta(accountId: string, row: BackupMetaRow): Promise<void>;
	listBackupMeta(accountId: string): Promise<BackupMetaRow[]>;
	getBackupMeta(accountId: string, packageId: string): Promise<BackupMetaRow | null>;
	upsertPushSubscription(accountId: string, sub: PushSubscriptionRow): Promise<void>;
	deletePushSubscription(endpoint: string): Promise<void>;
	deletePushSubscriptionForDevice(accountId: string, deviceId: string): Promise<void>;
	getPushSubscription(accountId: string, deviceId: string): Promise<PushSubscriptionRow | null>;
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
	unregisterPushSubscription(accountId: string, deviceId: string): Promise<void>;
	sendTestPush(accountId: string, deviceId: string): Promise<void>;
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

export class PushSubscriptionNotFoundError extends Error {
	constructor() {
		super('No push subscription for this device.');
	}
}
