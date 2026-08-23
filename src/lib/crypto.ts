export type CipherString = string;

export const VAULT_KDF_ITERATIONS = 600_000;
export const CHECK_PLAINTEXT = 'eisen-ok';
const AUTH_INFO = 'eisen-auth-v1';
const KEY_LENGTH = 256;

function requireSubtle(): SubtleCrypto {
	const subtle = globalThis.crypto?.subtle;
	if (!subtle) throw new Error('Web Crypto is not available.');
	return subtle;
}

export function toBase64(bytes: Uint8Array): string {
	const chunk = 8192;
	let result = '';
	for (let i = 0; i < bytes.length; i += chunk) {
		result += String.fromCharCode(...bytes.subarray(i, i + chunk));
	}
	return btoa(result);
}

export function fromBase64(s: string): Uint8Array {
	const binary = atob(s);
	const bytes = new Uint8Array(binary.length);
	for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
	return bytes;
}

async function pbkdf2Raw(
	password: string,
	salt: Uint8Array,
	iterations: number
): Promise<ArrayBuffer> {
	const subtle = requireSubtle();
	const imported = await subtle.importKey('raw', new TextEncoder().encode(password), 'PBKDF2', false, [
		'deriveBits'
	]);
	return subtle.deriveBits(
		{ name: 'PBKDF2', salt: new Uint8Array(salt), iterations, hash: 'SHA-256' },
		imported,
		KEY_LENGTH
	);
}

export async function deriveAuthVerifier(
	password: string,
	email: string,
	iterations = VAULT_KDF_ITERATIONS
): Promise<string> {
	const salt = new TextEncoder().encode(`${AUTH_INFO}:${email.trim().toLowerCase()}`);
	const bits = await pbkdf2Raw(password, salt, iterations);
	return toBase64(new Uint8Array(bits));
}

export async function deriveVaultKey(
	password: string,
	salt: Uint8Array,
	iterations = VAULT_KDF_ITERATIONS
): Promise<CryptoKey> {
	const subtle = requireSubtle();
	const imported = await subtle.importKey('raw', new TextEncoder().encode(password), 'PBKDF2', false, [
		'deriveKey'
	]);
	return subtle.deriveKey(
		{ name: 'PBKDF2', salt: new Uint8Array(salt), iterations, hash: 'SHA-256' },
		imported,
		{ name: 'AES-GCM', length: KEY_LENGTH },
		false,
		['encrypt', 'decrypt']
	);
}

export async function encrypt(plaintext: string, key: CryptoKey): Promise<CipherString> {
	const subtle = requireSubtle();
	const iv = crypto.getRandomValues(new Uint8Array(12));
	const ciphertext = new Uint8Array(
		await subtle.encrypt({ name: 'AES-GCM', iv }, key, new TextEncoder().encode(plaintext))
	);
	const combined = new Uint8Array(iv.length + ciphertext.length);
	combined.set(iv);
	combined.set(ciphertext, iv.length);
	return toBase64(combined);
}

export async function decrypt(packed: CipherString, key: CryptoKey): Promise<string> {
	const subtle = requireSubtle();
	const combined = fromBase64(packed);
	if (combined.length < 13) throw new Error('Invalid ciphertext.');
	const iv = new Uint8Array(combined.subarray(0, 12));
	const ciphertext = new Uint8Array(combined.subarray(12));
	const plaintext = await subtle.decrypt({ name: 'AES-GCM', iv }, key, ciphertext);
	return new TextDecoder().decode(plaintext);
}

export async function newKdfSalt(): Promise<Uint8Array> {
	return crypto.getRandomValues(new Uint8Array(16));
}

export async function makeCheckBlob(key: CryptoKey): Promise<CipherString> {
	return encrypt(CHECK_PLAINTEXT, key);
}

export async function verifyCheckBlob(checkBlob: CipherString, key: CryptoKey): Promise<boolean> {
	try {
		return (await decrypt(checkBlob, key)) === CHECK_PLAINTEXT;
	} catch {
		return false;
	}
}
