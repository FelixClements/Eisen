import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { requireUser } from '$lib/server/require-user';

export const POST: RequestHandler = async (event) => {
	const user = requireUser(event);
	const d1 = event.platform?.env?.DB;
	if (!d1) throw error(500, 'D1 binding not configured');

	const { endpoint, p256dh, auth } = (await event.request.json()) as {
		deviceId?: string;
		endpoint: string;
		p256dh: string;
		auth: string;
	};

	if (!endpoint || !p256dh || !auth) throw error(400, 'Missing subscription fields.');

	const id = crypto.randomUUID();
	await d1
		.prepare(
			`INSERT INTO push_subscriptions (id, user_id, endpoint, p256dh, auth, created_at)
			 VALUES (?, ?, ?, ?, ?, ?)
			 ON CONFLICT(endpoint) DO UPDATE SET
			   user_id = excluded.user_id,
			   p256dh = excluded.p256dh,
			   auth = excluded.auth`
		)
		.bind(id, user.id, endpoint, p256dh, auth, Date.now())
		.run();

	return json({ success: true });
};
