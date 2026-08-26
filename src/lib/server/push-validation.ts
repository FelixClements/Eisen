import { error } from '@sveltejs/kit';

const UUID_RE =
	/^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const BASE64URL_RE = /^[A-Za-z0-9_-]+$/;
const ONE_YEAR_MS = 365 * 24 * 60 * 60 * 1000;

/** Known browser push service hosts (RFC 8030). */
const PUSH_ENDPOINT_HOSTS = new Set([
	'fcm.googleapis.com',
	'updates.push.services.mozilla.com',
	'web.push.apple.com',
	'push.apple.com',
	'notify.windows.com'
]);

export function assertDeviceId(deviceId: string): void {
	if (!UUID_RE.test(deviceId)) throw error(400, 'Invalid deviceId.');
}

export function assertWakeAt(wakeAt: number, now = Date.now()): void {
	if (!Number.isFinite(wakeAt) || wakeAt <= 0) throw error(400, 'Invalid wakeAt.');
	if (wakeAt < now) throw error(400, 'wakeAt must be in the future.');
	if (wakeAt > now + ONE_YEAR_MS) throw error(400, 'wakeAt is too far in the future.');
}

export function assertPushEndpoint(endpoint: string): void {
	let url: URL;
	try {
		url = new URL(endpoint);
	} catch {
		throw error(400, 'Invalid push endpoint.');
	}
	if (url.protocol !== 'https:') throw error(400, 'Push endpoint must use HTTPS.');
	if (!PUSH_ENDPOINT_HOSTS.has(url.hostname)) {
		throw error(400, 'Push endpoint host is not allowed.');
	}
}

export function assertPushKeys(p256dh: string, auth: string): void {
	if (!BASE64URL_RE.test(p256dh) || p256dh.length < 80 || p256dh.length > 200) {
		throw error(400, 'Invalid p256dh key.');
	}
	if (!BASE64URL_RE.test(auth) || auth.length < 16 || auth.length > 64) {
		throw error(400, 'Invalid auth key.');
	}
}
