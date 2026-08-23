import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { requireUser } from '$lib/server/require-user';
import { mirrorFromEvent } from '$lib/server/mirror-from-event';

export const POST: RequestHandler = async (event) => {
	const user = requireUser(event);
	const mirror = mirrorFromEvent(event);
	const { endpoint, p256dh, auth } = (await event.request.json()) as {
		endpoint: string;
		p256dh: string;
		auth: string;
	};
	if (!endpoint || !p256dh || !auth) throw error(400, 'Missing subscription fields.');
	await mirror.registerPushSubscription(user.id, { endpoint, p256dh, auth });
	return json({ success: true });
};
