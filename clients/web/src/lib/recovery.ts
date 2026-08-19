import { browser } from '$app/environment';
import { db, getDeviceId, type Task, type VaultState } from './db';
import { deriveMasterKey, encrypt, decrypt, toBase64, fromBase64 } from './crypto';

export interface RecoveryPackage {
	version: number;
	userId: string;
	vaultId: string;
	kdfSalt: string;
	ciphertext: string;
	createdAt: number;
}

export async function exportRecoveryPackage(userId: string, passphrase: string): Promise<Blob> {
	if (!browser) throw new Error('Recovery export is browser-only.');

	const vault = await db.vaults.get(userId);
	if (!vault) throw new Error('No vault to back up.');

	const tasks = await db.tasks.where('userId').equals(userId).toArray();
	const payload = JSON.stringify({ vault, tasks });

	const kdfSalt = crypto.getRandomValues(new Uint8Array(16));
	const key = await deriveMasterKey(passphrase, kdfSalt);
	const ciphertext = await encrypt(payload, key);

	const pkg: RecoveryPackage = {
		version: 1,
		userId: vault.userId,
		vaultId: vault.vaultId,
		kdfSalt: toBase64(kdfSalt),
		ciphertext,
		createdAt: Date.now()
	};

	return new Blob([JSON.stringify(pkg)], { type: 'application/eisen-recovery' });
}

export async function importRecoveryPackage(
	userId: string,
	file: File,
	passphrase: string
): Promise<void> {
	if (!browser) throw new Error('Recovery import is browser-only.');

	const pkg: RecoveryPackage = JSON.parse(await file.text());
	if (pkg.version !== 1) throw new Error('Unsupported recovery package version.');
	if (pkg.userId !== userId) throw new Error('Recovery package belongs to a different account.');

	const key = await deriveMasterKey(passphrase, fromBase64(pkg.kdfSalt));
	let plaintext: string;
	try {
		plaintext = await decrypt(pkg.ciphertext, key);
	} catch {
		throw new Error('Wrong passphrase or corrupted recovery package.');
	}

	const { vault, tasks } = JSON.parse(plaintext) as { vault: VaultState; tasks: Task[] };

	await db.transaction('rw', db.vaults, db.tasks, async () => {
		await db.vaults.put({ ...vault, userId });
		for (const task of tasks) {
			await db.tasks.put({ ...task, userId });
		}
	});

	const deviceId = await getDeviceId(userId);
	await db.deviceState.put({ deviceId, userId, lastSyncAt: null });
}

export async function backupToCloud(userId: string, passphrase: string): Promise<string> {
	const blob = await exportRecoveryPackage(userId, passphrase);
	const packageText = await blob.text();
	const packageId = crypto.randomUUID();
	const deviceId = await getDeviceId(userId);

	const response = await fetch('/api/backup', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ packageId, packageText, deviceId })
	});

	if (!response.ok) throw new Error(`Cloud backup failed: ${response.status}`);
	const data = (await response.json()) as { packageId: string };
	return data.packageId;
}

export async function listCloudBackups(): Promise<{ packageId: string; createdAt: number }[]> {
	const response = await fetch('/api/backup');
	if (!response.ok) throw new Error(`Failed to list backups: ${response.status}`);
	const data = (await response.json()) as { backups: { packageId: string; createdAt: number }[] };
	return data.backups;
}
