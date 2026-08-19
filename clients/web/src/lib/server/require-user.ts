import { error } from '@sveltejs/kit';
import type { RequestEvent } from '@sveltejs/kit';

export function requireUser(event: RequestEvent) {
	const user = event.locals.user;
	if (!user) {
		throw error(401, 'Authentication required.');
	}
	return user;
}
