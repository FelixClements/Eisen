import { browser } from '$app/environment';
import { db, getAccount } from './db';
import { exportRecoveryPackage } from './recovery';

export async function backupToCloud(password: string): Promise<string> {
	if (!browser) throw new Error('Cloud backup is browser-only.');

	const account = await getAccount();
	if (!account) throw new Error('No account to back up.');

	const device = await db.deviceState.toCollection().first();
	if (!device) throw new Error('No device state.');

	const blob = await exportRecoveryPackage(password);
	const packageId = crypto.randomUUID();
	const packageText = await blob.text();

	const res = await fetch('/api/backup', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({
			ownerId: account.ownerId,
			deviceId: device.deviceId,
			packageId,
			packageText
		})
	});

	if (!res.ok) {
		throw new Error('Cloud backup failed.');
	}

	return packageId;
}

export interface BackupRecord {
	packageId: string;
	createdAt: number;
}

export async function listCloudBackups(): Promise<BackupRecord[]> {
	if (!browser) throw new Error('Cloud backup is browser-only.');

	const account = await getAccount();
	if (!account) throw new Error('No account.');

	const device = await db.deviceState.toCollection().first();
	if (!device) throw new Error('No device state.');

	const res = await fetch(`/api/backup?ownerId=${account.ownerId}&deviceId=${device.deviceId}`);
	if (!res.ok) {
		throw new Error('Failed to list cloud backups.');
	}

	const { backups } = (await res.json()) as { backups: BackupRecord[] };
	return backups;
}

export async function downloadCloudBackup(packageId: string): Promise<string> {
	if (!browser) throw new Error('Cloud backup is browser-only.');

	const account = await getAccount();
	if (!account) throw new Error('No account.');

	const device = await db.deviceState.toCollection().first();
	if (!device) throw new Error('No device state.');

	const res = await fetch(`/api/backup/${packageId}?ownerId=${account.ownerId}&deviceId=${device.deviceId}`);
	if (!res.ok) {
		throw new Error('Failed to download cloud backup.');
	}

	return await res.text();
}
