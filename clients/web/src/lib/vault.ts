import { browser } from '$app/environment';
import { goto } from '$app/navigation';
import { writable } from 'svelte/store';
import type { Writable } from 'svelte/store';
import { deriveMasterKey, encrypt, decrypt } from './crypto';
import { db, getVault, createVaultRecord } from './db';

const VALIDATION_VALUE = 'eisen-validation-value';

export const masterKey: Writable<CryptoKey | null> = writable(null);
export const vaultUserId: Writable<string | null> = writable(null);

export async function vaultExists(userId: string): Promise<boolean> {
	if (!browser) return false;
	return !!(await getVault(userId));
}

export async function setupVault(userId: string, passphrase: string, keepSignedIn = false): Promise<void> {
	if (!browser) throw new Error('Vault setup is browser-only.');
	const existing = await getVault(userId);
	if (existing) throw new Error('Vault already exists.');

	const salt = crypto.getRandomValues(new Uint8Array(16));
	const key = await deriveMasterKey(passphrase, salt);
	const encryptedValidation = await encrypt(VALIDATION_VALUE, key);
	await createVaultRecord(userId, encryptedValidation, salt);

	masterKey.set(key);
	vaultUserId.set(userId);
	if (keepSignedIn) await persistSession(userId, key);
}

export async function unlockVault(userId: string, passphrase: string, keepSignedIn = false): Promise<void> {
	if (!browser) throw new Error('Unlock is browser-only.');
	const vault = await getVault(userId);
	if (!vault) throw new Error('No vault found. Set up your vault first.');

	const salt = Uint8Array.from(atob(vault.deviceSalt), (c) => c.charCodeAt(0));
	const key = await deriveMasterKey(passphrase, salt);

	try {
		const decrypted = await decrypt(vault.validationValue, key);
		if (decrypted !== VALIDATION_VALUE) throw new Error('Incorrect passphrase');
	} catch {
		throw new Error('Incorrect passphrase');
	}

	masterKey.set(key);
	vaultUserId.set(userId);
	if (keepSignedIn) await persistSession(userId, key);
}

export async function persistSession(userId: string, key: CryptoKey): Promise<void> {
	if (!browser) return;
	await db.sessions.put({ id: 'current', userId, key, createdAt: Date.now() });
}

export async function tryAutoUnlock(userId: string): Promise<void> {
	if (!browser) return;
	const vault = await getVault(userId);
	if (!vault) return;

	const session = await db.sessions.get('current');
	if (!session || session.userId !== userId) return;

	try {
		const decrypted = await decrypt(vault.validationValue, session.key);
		if (decrypted !== VALIDATION_VALUE) {
			await clearSession();
			return;
		}
		masterKey.set(session.key);
		vaultUserId.set(userId);
	} catch {
		await clearSession();
	}
}

export async function clearSession(): Promise<void> {
	if (!browser) return;
	await db.sessions.delete('current');
}

export async function lockVault(): Promise<void> {
	masterKey.set(null);
	vaultUserId.set(null);
	await clearSession();
	if (browser) await goto('/', { replaceState: true });
}
