import { error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { requireUser } from '$lib/server/require-user';
import { fanoutWatchBindings, proxyAccountWatch } from '$lib/server/fanout-client';

export const GET: RequestHandler = async (event) => {
	const user = requireUser(event);
	const bindings = fanoutWatchBindings(event.platform?.env);
	if (!bindings) throw error(404, 'Fanout not configured');
	if (event.request.headers.get('Upgrade') !== 'websocket') {
		return new Response(null, { status: 204 });
	}
	return proxyAccountWatch(bindings, event.request, user.id);
};
