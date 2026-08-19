/// <reference lib="webworker" />
import { clientsClaim } from 'workbox-core';
import { precacheAndRoute } from 'workbox-precaching';

declare const self: ServiceWorkerGlobalScope;

precacheAndRoute(self.__WB_MANIFEST);
clientsClaim();

self.addEventListener('push', (event) => {
	event.waitUntil(handlePush(event));
});

async function handlePush(event: PushEvent) {
	let data: { type?: string; userId?: string } = {};
	try {
		data = event.data?.json() ?? {};
	} catch {
		data = { type: 'wake' };
	}

	if (data.type !== 'wake') {
		await self.registration.showNotification('Eisen', { body: 'You have a reminder.' });
		return;
	}

	const due = await getDueReminders();
	for (const reminder of due) {
		await self.registration.showNotification('Eisen reminder', {
			body: reminder.title,
			tag: reminder.id,
			data: { url: `/task/${reminder.id}` }
		});
	}

	if (due.length === 0) {
		await self.registration.showNotification('Eisen', { body: 'Check your reminders.' });
	}
}

self.addEventListener('notificationclick', (event) => {
	event.notification.close();
	const url = (event.notification.data as { url?: string })?.url ?? '/';
	event.waitUntil(self.clients.openWindow(url));
});

interface ReminderRow {
	id: string;
	title: string;
}

async function getDueReminders(): Promise<ReminderRow[]> {
	// Service worker reads reminder metadata from IndexedDB via a client message when available.
	const clients = await self.clients.matchAll({ type: 'window', includeUncontrolled: true });
	if (clients.length === 0) return [];

	return new Promise((resolve) => {
		const channel = new MessageChannel();
		channel.port1.onmessage = (e) => resolve((e.data as ReminderRow[]) ?? []);
		clients[0].postMessage({ type: 'GET_DUE_REMINDERS' }, [channel.port2]);
		setTimeout(() => resolve([]), 2000);
	});
}
