import type { RemindersPort } from './ports';

function urlBase64ToUint8Array(base64String: string): Uint8Array {
	const padding = '='.repeat((4 - (base64String.length % 4)) % 4);
	const base64 = (base64String + padding).replace(/-/g, '+').replace(/_/g, '/');
	const raw = atob(base64);
	const output = new Uint8Array(raw.length);
	for (let i = 0; i < raw.length; i++) output[i] = raw.charCodeAt(i);
	return output;
}

export function browserReminders(vapidPublicKey = ''): RemindersPort {
	const key = vapidPublicKey || import.meta.env.VITE_VAPID_PUBLIC_KEY || '';
	return {
		permission() {
			if (typeof Notification === 'undefined' || !('serviceWorker' in navigator)) return 'unsupported';
			return Notification.permission;
		},
		async request() {
			if (typeof Notification === 'undefined') return 'unsupported';
			const perm = await Notification.requestPermission();
			if (perm === 'granted') return 'granted';
			if (perm === 'denied') return 'denied';
			return 'unsupported';
		},
		async subscribe() {
			if (!key || !('serviceWorker' in navigator) || !('PushManager' in window)) {
				return null;
			}
			const registration = await navigator.serviceWorker.ready;
			const subscription = await registration.pushManager.subscribe({
				userVisibleOnly: true,
				applicationServerKey: urlBase64ToUint8Array(key) as BufferSource
			});
			const json = subscription.toJSON();
			if (!json.endpoint || !json.keys?.p256dh || !json.keys?.auth) return null;
			return { endpoint: json.endpoint, p256dh: json.keys.p256dh, auth: json.keys.auth };
		},
		async unsubscribe() {
			if (!('serviceWorker' in navigator) || !('PushManager' in window)) return;
			const registration = await navigator.serviceWorker.ready;
			const subscription = await registration.pushManager.getSubscription();
			await subscription?.unsubscribe();
		},
		vapidReady() {
			return Boolean(key);
		}
	};
}
