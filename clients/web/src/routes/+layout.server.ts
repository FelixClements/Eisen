import { redirect } from '@sveltejs/kit';
import type { LayoutServerLoad } from './$types';

const publicPaths = new Set(['/sign-in', '/sign-up']);

export const load: LayoutServerLoad = async ({ locals, url }) => {
	const user = locals.user;
	const path = url.pathname;

	if (!user && !publicPaths.has(path)) {
		redirect(303, '/sign-in');
	}

	if (user && publicPaths.has(path)) {
		redirect(303, '/');
	}

	return { user };
};
