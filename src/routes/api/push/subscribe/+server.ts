import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { requireUser } from '$lib/server/require-user';
import { mirrorFromEvent } from '$lib/server/mirror-from-event';
import { PushSubscriptionConflictError } from '$lib/server/encrypted-mirror';
import { assertDeviceId, assertPushEndpoint, assertPushKeys } from '$lib/server/push-validation';

export const POST: RequestHandler = async (event) => {
	const user = requireUser(event);
	const mirror = mirrorFromEvent(event);
	const { deviceId, endpoint, p256dh, auth } = (await event.request.json()) as {
		deviceId: string;
		endpoint: string;
		p256dh: string;
		auth: string;
	};
	if (!deviceId || !endpoint || !p256dh || !auth) throw error(400, 'Missing subscription fields.');
	assertDeviceId(deviceId);
	assertPushEndpoint(endpoint);
	assertPushKeys(p256dh, auth);
	try {
		await mirror.registerPushSubscription(user.id, { deviceId, endpoint, p256dh, auth });
	} catch (err) {
		if (err instanceof PushSubscriptionConflictError) throw error(403, err.message);
		throw err;
	}
	return json({ success: true });
};
