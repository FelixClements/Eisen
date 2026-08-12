import { browser } from '$app/environment';
import { deriveMasterKey } from './crypto';

const SALT_KEY = 'eisen-salt';

function getSalt(): Uint8Array {
	if (!browser) throw new Error('Salt can only be managed in the browser.');
	const stored = localStorage.getItem(SALT_KEY);
	if (stored) {
		const bytes = new Uint8Array(16);
		const parsed = atob(stored);
		for (let i = 0; i < parsed.length; i++) {
			bytes[i] = parsed.charCodeAt(i);
		}
		return bytes;
	}
	const salt = new Uint8Array(crypto.getRandomValues(new Uint8Array(16)));
	const b64 = btoa(String.fromCharCode(...salt));
	localStorage.setItem(SALT_KEY, b64);
	return salt;
}

export async function unlock(password: string): Promise<CryptoKey> {
	const salt = getSalt();
	return deriveMasterKey(password, salt);
}
