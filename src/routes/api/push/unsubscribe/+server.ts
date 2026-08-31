import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { requireUser } from '$lib/server/require-user';
import { mirrorFromEvent } from '$lib/server/mirror-from-event';
import { assertDeviceId } from '$lib/server/push-validation';

export const POST: RequestHandler = async (event) => {
	const user = requireUser(event);
	const mirror = mirrorFromEvent(event);
	const { deviceId } = (await event.request.json()) as { deviceId: string };
	if (!deviceId) throw error(400, 'Missing deviceId.');
	assertDeviceId(deviceId);
	await mirror.unregisterPushSubscription(user.id, deviceId);
	return json({ success: true });
};
