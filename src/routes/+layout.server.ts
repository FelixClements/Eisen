import { redirect } from '@sveltejs/kit';
import type { LayoutServerLoad } from './$types';
import { vapidApplicationServerKey } from '$lib/server/vapid-public';

const publicPaths = new Set(['/sign-in', '/sign-up']);

export const load: LayoutServerLoad = async ({ locals, url, platform }) => {
	const user = locals.user;
	const path = url.pathname;

	if (!user && !publicPaths.has(path)) {
		redirect(303, '/sign-in');
	}

	return {
		user,
		vapidPublicKey: vapidApplicationServerKey(platform?.env)
	};
};
