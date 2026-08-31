export type BackupRef = { id: string; createdAt: number };

export type VaultParams = {
	salt: string;
	checkBlob: string;
};

export type SyncRecord = {
	recordId: string;
	encryptedBlob: string;
	modifiedAt: number;
	deviceId: string;
	syncVersion?: number;
	deleted: number;
};

export type SyncPushBatch = {
	lastVersion: number;
	changes: SyncRecord[];
};

export type SyncPullBatch = {
	changes: SyncRecord[];
	lastVersion: number;
};

export class VaultParamsExistError extends Error {
	constructor() {
		super('Vault params already exist for this account.');
	}
}
