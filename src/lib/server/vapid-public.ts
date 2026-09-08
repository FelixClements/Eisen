import { fromBase64, toBase64 } from '$lib/crypto';

function fromBase64Url(s: string): Uint8Array {
	const pad = '='.repeat((4 - (s.length % 4)) % 4);
	return fromBase64(s.replace(/-/g, '+').replace(/_/g, '/') + pad);
}

function toBase64Url(bytes: Uint8Array): string {
	return toBase64(bytes).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

export function applicationServerKeyFromPrivateJwk(raw: string | undefined): string | null {
	if (!raw?.trim()) return null;
	try {
		const jwk = JSON.parse(raw) as JsonWebKey;
		if (!jwk.x || !jwk.y) return null;
		const x = fromBase64Url(jwk.x);
		const y = fromBase64Url(jwk.y);
		if (x.length !== 32 || y.length !== 32) return null;
		const uncompressed = new Uint8Array(65);
		uncompressed[0] = 0x04;
		uncompressed.set(x, 1);
		uncompressed.set(y, 33);
		return toBase64Url(uncompressed);
	} catch {
		return null;
	}
}

export function vapidApplicationServerKey(env?: {
	VAPID_PUBLIC_KEY?: string;
	VAPID_PRIVATE_KEY?: string;
}): string {
	const explicit = env?.VAPID_PUBLIC_KEY?.trim();
	if (explicit) return explicit;
	return applicationServerKeyFromPrivateJwk(env?.VAPID_PRIVATE_KEY) ?? '';
}
