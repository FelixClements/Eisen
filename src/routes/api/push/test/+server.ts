import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { requireUser } from '$lib/server/require-user';
import { mirrorFromEvent } from '$lib/server/mirror-from-event';
import { PushSubscriptionNotFoundError } from '$lib/server/encrypted-mirror';
import { PushSendError } from '$lib/server/adapters/web-push';
import { assertDeviceId } from '$lib/server/push-validation';

export const POST: RequestHandler = async (event) => {
	const user = requireUser(event);
	const mirror = mirrorFromEvent(event);
	const { deviceId } = (await event.request.json()) as { deviceId: string };
	if (!deviceId) throw error(400, 'Missing deviceId.');
	assertDeviceId(deviceId);
	try {
		await mirror.sendTestPush(user.id, deviceId);
	} catch (err) {
		if (err instanceof PushSubscriptionNotFoundError) {
			throw error(404, 'No push subscription for this device. Enable push reminders first.');
		}
		if (err instanceof PushSendError) {
			if (err.status === 503) {
				throw error(503, 'Push is not configured (VAPID private key missing).');
			}
			throw error(502, `Push provider rejected the test: HTTP ${err.status}`);
		}
		throw err;
	}
	return json({ success: true });
};
