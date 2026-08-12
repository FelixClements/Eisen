import { browser } from '$app/environment';

export interface PackedCipher {
	iv: string;
	data: string;
}

export type CipherString = string;

const PBKDF2_ITERATIONS = 600_000;
const KEY_LENGTH = 256;

export function toBase64(bytes: Uint8Array): string {
	const chunk = 8192;
	let result = '';
	for (let i = 0; i < bytes.length; i += chunk) {
		const slice = bytes.subarray(i, i + chunk);
		result += String.fromCharCode(...slice);
	}
	return btoa(result);
}

export function fromBase64(s: string): Uint8Array {
	const binary = atob(s);
	const bytes = new Uint8Array(binary.length);
	for (let i = 0; i < binary.length; i++) {
		bytes[i] = binary.charCodeAt(i);
	}
	return bytes;
}

export async function deriveMasterKey(password: string, salt: Uint8Array): Promise<CryptoKey> {
	if (!browser) throw new Error('Crypto is only available in the browser.');
	const encoder = new TextEncoder();
	const safeSalt = new Uint8Array(salt);
	const imported = await crypto.subtle.importKey('raw', new Uint8Array(encoder.encode(password)), 'PBKDF2', false, [
		'deriveKey'
	]);
	return crypto.subtle.deriveKey(
		{
			name: 'PBKDF2',
			salt: safeSalt,
			iterations: PBKDF2_ITERATIONS,
			hash: 'SHA-256'
		},
		imported,
		{ name: 'AES-GCM', length: KEY_LENGTH },
		false,
		['encrypt', 'decrypt']
	);
}

export async function encrypt(plaintext: string, key: CryptoKey): Promise<CipherString> {
	if (!browser) throw new Error('Crypto is only available in the browser.');
	const encoder = new TextEncoder();
	const iv = new Uint8Array(crypto.getRandomValues(new Uint8Array(12)));
	const ciphertext = new Uint8Array(
		await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, key, encoder.encode(plaintext))
	);
	const combined = new Uint8Array(iv.length + ciphertext.length);
	combined.set(iv);
	combined.set(ciphertext, iv.length);
	return toBase64(combined);
}

export async function decrypt(packed: CipherString, key: CryptoKey): Promise<string> {
	if (!browser) throw new Error('Crypto is only available in the browser.');
	const combined = fromBase64(packed);
	if (combined.length < 13) throw new Error('Invalid ciphertext.');
	const iv = new Uint8Array(combined.subarray(0, 12));
	const ciphertext = new Uint8Array(combined.subarray(12));
	const decoder = new TextDecoder();
	const plaintext = await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, key, ciphertext);
	return decoder.decode(plaintext);
}
