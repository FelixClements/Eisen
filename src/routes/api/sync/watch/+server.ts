import { error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { requireUser } from '$lib/server/require-user';
import { proxyAccountWatch } from '$lib/server/fanout-client';

export const GET: RequestHandler = async (event) => {
	const user = requireUser(event);
	const env = event.platform?.env;
	if (!env?.SYNC_FANOUT || !env.FANOUT_SECRET) throw error(404, 'Fanout not configured');
	if (event.request.headers.get('Upgrade') !== 'websocket') {
		return new Response(null, { status: 204 });
	}
	return proxyAccountWatch(
		{ SYNC_FANOUT: env.SYNC_FANOUT, FANOUT_SECRET: env.FANOUT_SECRET },
		event.request,
		user.id
	);
};
