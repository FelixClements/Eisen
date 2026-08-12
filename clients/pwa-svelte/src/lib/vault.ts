import { browser } from '$app/environment';
import { writable } from 'svelte/store';
import type { Writable } from 'svelte/store';
import { deriveMasterKey, encrypt, decrypt } from './crypto';
import { db, getAccount, createAccountRecord } from './db';

const VALIDATION_VALUE = 'eisen-validation-value';

export const masterKey: Writable<CryptoKey | null> = writable(null);

export async function accountExists(): Promise<boolean> {
	if (!browser) return false;
	const account = await getAccount();
	return !!account;
}

export async function createAccount(password: string): Promise<void> {
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
}

export async function unlock(password: string): Promise<void> {
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
}

export async function lock(): Promise<void> {
	masterKey.set(null);
}
