import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
// @ts-expect-error web-push types may be missing in dev
import webpush from 'web-push';

export const GET: RequestHandler = async (event) => {
	const env = event.platform?.env;
	const d1 = env?.DB;
	if (!d1) throw error(500, 'D1 binding not configured');

	const publicKey = env.VAPID_PUBLIC_KEY;
	const privateKey = env.VAPID_PRIVATE_KEY;
	const subject = env.VAPID_SUBJECT ?? 'mailto:admin@eisen.app';

	if (!publicKey || !privateKey) {
		return json({ sent: 0, message: 'VAPID keys not configured' });
	}

	webpush.setVapidDetails(subject, publicKey, privateKey);

	const now = Date.now();
	const { results: due } = await d1
		.prepare(
			`SELECT ws.id, ws.user_id AS userId, ps.endpoint, ps.p256dh, ps.auth
			 FROM wake_schedules ws
			 JOIN push_subscriptions ps ON ps.user_id = ws.user_id
			 WHERE ws.sent = 0 AND ws.wake_at <= ?`
		)
		.bind(now)
		.all<{
			id: string;
			userId: string;
			endpoint: string;
			p256dh: string;
			auth: string;
		}>();

	let sent = 0;
	for (const row of due ?? []) {
		try {
			await webpush.sendNotification(
				{
					endpoint: row.endpoint,
					keys: { p256dh: row.p256dh, auth: row.auth }
				},
				JSON.stringify({ type: 'wake', userId: row.userId })
			);
			await d1.prepare('UPDATE wake_schedules SET sent = 1 WHERE id = ?').bind(row.id).run();
			sent++;
		} catch (e) {
			console.error('Push failed', e);
		}
	}

	return json({ sent });
};
