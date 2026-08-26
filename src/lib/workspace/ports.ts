import type { BackupRef, SyncRecord } from './types';

export type SyncPushBatch = {
	lastVersion: number;
	changes: SyncRecord[];
};

export type SyncPullBatch = {
	changes: SyncRecord[];
	lastVersion: number;
};

export type VaultParams = {
	salt: string;
	checkBlob: string;
};

export interface VaultParamsPort {
	getVaultParams(): Promise<VaultParams | null>;
	createVaultParams(params: VaultParams): Promise<void>;
}

export interface SyncPort {
	sync(batch: SyncPushBatch): Promise<SyncPullBatch>;
}

export interface BackupPort {
	putBackup(pkg: { id: string; text: string; deviceId: string }): Promise<BackupRef>;
	listBackups(): Promise<BackupRef[]>;
	getBackup(packageId: string): Promise<string>;
}

export interface WakePort {
	scheduleWake(w: { deviceId: string; wakeAt: number; nonce: string }): Promise<void>;
	registerPush(sub: {
		deviceId: string;
		endpoint: string;
		p256dh: string;
		auth: string;
	}): Promise<void>;
}

export type ReminderEnableResult =
	| 'granted'
	| 'denied'
	| 'unsupported'
	| 'vapid-missing'
	| 'subscribe-failed'
	| 'server-error';

export type CloudPort = VaultParamsPort & SyncPort & BackupPort & WakePort;

export interface RemindersPort {
	permission(): 'unsupported' | 'default' | 'granted' | 'denied';
	request(): Promise<'granted' | 'denied' | 'unsupported'>;
	subscribe(): Promise<{ endpoint: string; p256dh: string; auth: string } | null>;
}

export type Clock = {
	now(): number;
	uuid(): string;
};

export const systemClock: Clock = {
	now: () => Date.now(),
	uuid: () => crypto.randomUUID()
};

export const nullReminders: RemindersPort = {
	permission: () => 'unsupported',
	request: async () => 'unsupported',
	subscribe: async () => null
};
