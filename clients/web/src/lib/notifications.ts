import { browser } from '$app/environment';
import { db, getDeviceId } from './db';

const VAPID_PUBLIC_KEY = import.meta.env.VITE_VAPID_PUBLIC_KEY ?? '';

function urlBase64ToUint8Array(base64String: string): Uint8Array {
	const padding = '='.repeat((4 - (base64String.length % 4)) % 4);
	const base64 = (base64String + padding).replace(/-/g, '+').replace(/_/g, '/');
	const raw = atob(base64);
	const output = new Uint8Array(raw.length);
	for (let i = 0; i < raw.length; i++) output[i] = raw.charCodeAt(i);
	return output;
}

export async function subscribeToPush(userId: string): Promise<boolean> {
	if (!browser || !('serviceWorker' in navigator) || !('PushManager' in window)) return false;
	if (!VAPID_PUBLIC_KEY) {
		console.warn('VITE_VAPID_PUBLIC_KEY not set; push subscription skipped.');
		return false;
	}

	const permission = await Notification.requestPermission();
	if (permission !== 'granted') return false;

	const registration = await navigator.serviceWorker.ready;
	const subscription = await registration.pushManager.subscribe({
		userVisibleOnly: true,
		applicationServerKey: urlBase64ToUint8Array(VAPID_PUBLIC_KEY) as BufferSource
	});

	const json = subscription.toJSON();
	if (!json.endpoint || !json.keys?.p256dh || !json.keys?.auth) return false;

	const deviceId = await getDeviceId(userId);
	const response = await fetch('/api/push/subscribe', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({
			deviceId,
			endpoint: json.endpoint,
			p256dh: json.keys.p256dh,
			auth: json.keys.auth
		})
	});

	return response.ok;
}

export async function scheduleNextWake(userId: string): Promise<void> {
	if (!browser) return;

	const now = Date.now();
	const tasks = await db.tasks.where('userId').equals(userId).toArray();
	const upcoming = tasks
		.filter((t) => !t.deleted && !t.isCompleted && !t.isArchived && t.reminderAt && t.reminderAt > now)
		.sort((a, b) => (a.reminderAt ?? 0) - (b.reminderAt ?? 0));

	if (upcoming.length === 0) return;

	const next = upcoming[0];
	const deviceId = await getDeviceId(userId);
	const nonce = crypto.randomUUID();

	await fetch('/api/push/schedule', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ deviceId, wakeAt: next.reminderAt, nonce })
	});
}

export function notificationsSupported(): boolean {
	return browser && 'Notification' in window && 'serviceWorker' in navigator;
}
