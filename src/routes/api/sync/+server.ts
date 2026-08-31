import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { requireUser } from '$lib/server/require-user';
import { mirrorFromEvent } from '$lib/server/mirror-from-event';
import type { SyncRecord } from '$lib/sync/types';

export const POST: RequestHandler = async (event) => {
	const user = requireUser(event);
	const mirror = mirrorFromEvent(event);
	const body = (await event.request.json()) as {
		lastVersion: number;
		changes: SyncRecord[];
	};
	const result = await mirror.exchangeSync(user.id, {
		lastVersion: body.lastVersion ?? 0,
		changes: body.changes ?? []
	});
	return json(result);
};
