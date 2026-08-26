import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { requireCronSecret } from '$lib/server/require-cron';
import { createMirrorFromEnv } from '$lib/server/mirror-env';

export const GET: RequestHandler = async (event) => {
	requireCronSecret(event);
	const env = event.platform?.env;
	if (!env) throw error(500, 'Platform env not configured');
	const mirror = createMirrorFromEnv(env);
	const result = await mirror.dispatchDueWakes(Date.now());
	return json(result);
};
