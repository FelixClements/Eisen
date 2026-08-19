import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { requireUser } from '$lib/server/require-user';

export const POST: RequestHandler = async (event) => {
	const user = requireUser(event);
	const d1 = event.platform?.env?.DB;
	if (!d1) throw error(500, 'D1 binding not configured');

	const { deviceId, wakeAt, nonce } = (await event.request.json()) as {
		deviceId: string;
		wakeAt: number;
		nonce: string;
	};

	if (!deviceId || !wakeAt || !nonce) throw error(400, 'Missing schedule fields.');

	const id = crypto.randomUUID();
	await d1
		.prepare(
			`INSERT INTO wake_schedules (id, user_id, device_id, wake_at, nonce, sent)
			 VALUES (?, ?, ?, ?, ?, 0)`
		)
		.bind(id, user.id, deviceId, wakeAt, nonce)
		.run();

	return json({ success: true, id });
};
