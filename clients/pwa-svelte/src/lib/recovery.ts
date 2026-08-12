import { browser } from '$app/environment';
import { db, type Account, type Task } from './db';
import { deriveMasterKey, encrypt, decrypt, toBase64, fromBase64 } from './crypto';

export interface RecoveryPackage {
	version: number;
	ownerId: string;
	vaultId: string;
	kdfSalt: string;
	ciphertext: string;
	createdAt: number;
}

export async function exportRecoveryPackage(password: string): Promise<Blob> {
	if (!browser) throw new Error('Recovery package export is browser-only.');

	const account = await db.accounts.toCollection().first();
	if (!account) throw new Error('No account to back up.');

	const tasks = await db.tasks.toArray();
	const payload = JSON.stringify({ account, tasks });

	const kdfSalt = new Uint8Array(crypto.getRandomValues(new Uint8Array(16)));
	const key = await deriveMasterKey(password, kdfSalt);
	const ciphertext = await encrypt(payload, key);

	const pkg: RecoveryPackage = {
		version: 1,
		ownerId: account.ownerId,
		vaultId: account.vaultId,
		kdfSalt: toBase64(kdfSalt),
		ciphertext,
		createdAt: Date.now()
	};

	return new Blob([JSON.stringify(pkg)], { type: 'application/eisen-recovery' });
}

export async function importRecoveryPackage(file: File, password: string): Promise<void> {
	if (!browser) throw new Error('Recovery package import is browser-only.');

	const text = await file.text();
	const pkg: RecoveryPackage = JSON.parse(text);

	if (pkg.version !== 1) {
		throw new Error('Unsupported recovery package version.');
	}

	const kdfSalt = fromBase64(pkg.kdfSalt);
	const key = await deriveMasterKey(password, kdfSalt);

	let plaintext: string;
	try {
		plaintext = await decrypt(pkg.ciphertext, key);
	} catch {
		throw new Error('Wrong passphrase or corrupted recovery package.');
	}

	const { account, tasks } = JSON.parse(plaintext) as { account: Account; tasks: Task[] };

	if (account.ownerId !== pkg.ownerId) {
		throw new Error('Recovery package owner ID mismatch.');
	}

	await db.transaction('rw', db.accounts, db.tasks, async () => {
		await db.accounts.clear();
		await db.tasks.clear();
		await db.accounts.add(account);
		await db.tasks.bulkAdd(tasks);
	});

	await db.deviceState.clear();
	await db.deviceState.add({
		deviceId: crypto.randomUUID(),
		ownerId: account.ownerId,
		lastSyncAt: null
	});
}
