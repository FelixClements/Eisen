import { createAuth } from '$lib/server/auth';
import { svelteKitHandler } from 'better-auth/svelte-kit';
import { building } from '$app/environment';
import type { Handle } from '@sveltejs/kit';

export const handle: Handle = async ({ event, resolve }) => {
	const env = event.platform?.env;
	if (!env?.DB) {
		return resolve(event);
	}

	const secret = env.BETTER_AUTH_SECRET ?? 'dev-secret-change-in-production-min-32-chars!!';
	const baseURL = env.BETTER_AUTH_URL ?? event.url.origin;
	const auth = createAuth(env.DB, secret, baseURL);

	try {
		const session = await auth.api.getSession({ headers: event.request.headers });
		if (session) {
			event.locals.session = session.session;
			event.locals.user = session.user;
		}
	} catch (err) {
		console.error('getSession failed:', err);
	}

	// Only delegate to Better Auth handler for auth API routes (avoids OTEL errors on pages)
	if (event.url.pathname.startsWith('/api/auth')) {
		return svelteKitHandler({ event, resolve, auth, building });
	}

	return resolve(event);
};
