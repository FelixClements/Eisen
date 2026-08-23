import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { requireUser } from '$lib/server/require-user';
import { mirrorFromEvent } from '$lib/server/mirror-from-event';

export const POST: RequestHandler = async (event) => {
	const user = requireUser(event);
	const mirror = mirrorFromEvent(event);
	const { deviceId, wakeAt, nonce } = (await event.request.json()) as {
		deviceId: string;
		wakeAt: number;
		nonce: string;
	};
	if (!deviceId || !wakeAt || !nonce) throw error(400, 'Missing schedule fields.');
	const result = await mirror.scheduleWake(user.id, { deviceId, wakeAt, nonce });
	return json({ success: true, id: result.scheduleId });
};
