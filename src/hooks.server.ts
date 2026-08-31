import { createAuth } from '$lib/server/auth';
import { requireAuthSecret, requireAuthUrl } from '$lib/server/auth-secret';
import { svelteKitHandler } from 'better-auth/svelte-kit';
import { building } from '$app/environment';
import { error } from '@sveltejs/kit';
import type { Handle } from '@sveltejs/kit';

export const handle: Handle = async ({ event, resolve }) => {
	const env = event.platform?.env;
	if (!env?.DB) {
		return resolve(event);
	}

	let secret: string;
	let baseURL: string;
	try {
		secret = requireAuthSecret(env.BETTER_AUTH_SECRET);
		baseURL = requireAuthUrl(env.BETTER_AUTH_URL);
	} catch (err) {
		const message = err instanceof Error ? err.message : 'Auth is not configured';
		throw error(503, message);
	}
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
