import { describe, expect, it } from 'vitest';
import {
	applicationServerKeyFromPrivateJwk,
	vapidApplicationServerKey
} from './vapid-public';

function fromBase64Url(s: string): Uint8Array {
	const pad = '='.repeat((4 - (s.length % 4)) % 4);
	const b64 = s.replace(/-/g, '+').replace(/_/g, '/') + pad;
	const binary = atob(b64);
	const bytes = new Uint8Array(binary.length);
	for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
	return bytes;
}

describe('vapidApplicationServerKey', () => {
	it('returns an explicit public key when set', () => {
		expect(vapidApplicationServerKey({ VAPID_PUBLIC_KEY: 'abc' })).toBe('abc');
	});

	it('returns empty when neither key is set', () => {
		expect(vapidApplicationServerKey({})).toBe('');
	});

	it('derives the uncompressed P-256 public key from a private JWK', async () => {
		const pair = await crypto.subtle.generateKey({ name: 'ECDSA', namedCurve: 'P-256' }, true, [
			'sign',
			'verify'
		]);
		const jwk = await crypto.subtle.exportKey('jwk', pair.privateKey);
		const encoded = applicationServerKeyFromPrivateJwk(JSON.stringify(jwk));
		expect(encoded).toBeTruthy();
		const bytes = fromBase64Url(encoded!);
		expect(bytes.length).toBe(65);
		expect(bytes[0]).toBe(0x04);
		expect(vapidApplicationServerKey({ VAPID_PRIVATE_KEY: JSON.stringify(jwk) })).toBe(encoded);
	});
});
