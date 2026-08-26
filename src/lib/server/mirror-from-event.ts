import { error } from '@sveltejs/kit';
import type { RequestEvent } from '@sveltejs/kit';
import { createMirrorFromEnv } from './mirror-env';

export function mirrorFromEvent(event: RequestEvent) {
	const env = event.platform?.env;
	if (!env) throw error(500, 'Platform env not configured');
	return createMirrorFromEnv(env);
}
