import { afterEach, describe, expect, it, vi } from 'vitest';
import { webPushDispatch } from './web-push';

function toBase64Url(bytes: Uint8Array): string {
	let binary = '';
	for (const b of bytes) binary += String.fromCharCode(b);
	return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function jwtPayload(authorization: string): { aud: string; exp: number; sub: string } {
	const token = /t=([^,\s]+)/.exec(authorization)?.[1];
	if (!token) throw new Error('missing VAPID JWT');
	const payload = token.split('.')[1] ?? '';
	const pad = '='.repeat((4 - (payload.length % 4)) % 4);
	return JSON.parse(atob(payload.replace(/-/g, '+').replace(/_/g, '/') + pad)) as {
		aud: string;
		exp: number;
		sub: string;
	};
}

async function fakeSubscription() {
	const ecdh = await crypto.subtle.generateKey({ name: 'ECDH', namedCurve: 'P-256' }, true, [
		'deriveBits'
	]);
	const p256dh = toBase64Url(new Uint8Array(await crypto.subtle.exportKey('raw', ecdh.publicKey)));
	const auth = toBase64Url(crypto.getRandomValues(new Uint8Array(16)));
	return {
		deviceId: 'device-1',
		endpoint: 'https://fcm.googleapis.com/fcm/send/fake-subscription',
		p256dh,
		auth
	};
}

describe('webPushDispatch', () => {
	afterEach(() => {
		vi.unstubAllGlobals();
	});

	it('signs a VAPID JWT that expires strictly under 24 hours', async () => {
		const pair = await crypto.subtle.generateKey({ name: 'ECDSA', namedCurve: 'P-256' }, true, [
			'sign',
			'verify'
		]);
		const jwk = await crypto.subtle.exportKey('jwk', pair.privateKey);
		let authorization = '';
		vi.stubGlobal(
			'fetch',
			vi.fn(async (_url: string, init?: RequestInit) => {
				authorization = new Headers(init?.headers).get('Authorization') ?? '';
				return new Response(null, { status: 201 });
			})
		);

		const before = Math.floor(Date.now() / 1000);
		const dispatch = webPushDispatch({
			privateKeyJwk: JSON.stringify(jwk),
			subject: 'mailto:test@example.com'
		});
		await dispatch.send(await fakeSubscription(), JSON.stringify({ type: 'test' }));

		const payload = jwtPayload(authorization);
		expect(payload.aud).toBe('https://fcm.googleapis.com');
		expect(payload.sub).toBe('mailto:test@example.com');
		expect(payload.exp).toBeLessThan(before + 24 * 60 * 60);
	});
});
