import { browser } from '$app/environment';
import { goto } from '$app/navigation';
import { writable } from 'svelte/store';
import type { Writable } from 'svelte/store';
import { deriveMasterKey, encrypt, decrypt } from './crypto';
import { db, getAccount, createAccountRecord } from './db';
import { enrollDevice } from './enrollment';

const VALIDATION_VALUE = 'eisen-validation-value';

export const masterKey: Writable<CryptoKey | null> = writable(null);

export async function accountExists(): Promise<boolean> {
	if (!browser) return false;
	const account = await getAccount();
	return !!account;
}

export async function createAccount(password: string, keepSignedIn = false): Promise<void> {
	if (!browser) throw new Error('Accounts can only be created in the browser.');
	const existing = await getAccount();
	if (existing) {
		throw new Error('An account already exists on this device.');
	}

	const salt = new Uint8Array(crypto.getRandomValues(new Uint8Array(16)));
	const key = await deriveMasterKey(password, salt);
	const encryptedValidation = await encrypt(VALIDATION_VALUE, key);

	const account = await createAccountRecord(encryptedValidation, salt);
	if (!account) throw new Error('Failed to create account.');

	masterKey.set(key);
	if (keepSignedIn) {
		await persistSession(key);
	}

	enrollDevice(account).catch((err) => {
		console.warn('First-device cloud enrollment failed; falling back to offline-only.', err);
	});
}

export async function unlock(password: string, keepSignedIn = false): Promise<void> {
	if (!browser) throw new Error('Can only unlock in the browser.');
	const account = await getAccount();
	if (!account) {
		throw new Error('No account found. Create one first.');
	}

	const salt = Uint8Array.from(atob(account.deviceSalt), (c) => c.charCodeAt(0));
	const key = await deriveMasterKey(password, salt);

	try {
		const decrypted = await decrypt(account.validationValue, key);
		if (decrypted !== VALIDATION_VALUE) {
			throw new Error('Incorrect passphrase');
		}
	} catch {
		throw new Error('Incorrect passphrase');
	}

	masterKey.set(key);
	if (keepSignedIn) {
		await persistSession(key);
	}
}

export async function persistSession(key: CryptoKey): Promise<void> {
	if (!browser) return;
	await db.sessions.put({ id: 'current', key, createdAt: Date.now() });
}

export async function tryAutoUnlock(): Promise<void> {
	if (!browser) return;
	const account = await getAccount();
	if (!account) return;

	const session = await db.sessions.get('current');
	if (!session) return;

	try {
		const decrypted = await decrypt(account.validationValue, session.key);
		if (decrypted !== VALIDATION_VALUE) {
			await clearSession();
			return;
		}
		masterKey.set(session.key);
	} catch {
		await clearSession();
	}
}

export async function clearSession(): Promise<void> {
	if (!browser) return;
	await db.sessions.delete('current');
}

export async function lock(): Promise<void> {
	masterKey.set(null);
	await clearSession();
	if (browser) {
		await goto('/', { replaceState: true });
	}
}
