import { error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { requireUser } from '$lib/server/require-user';
import { mirrorFromEvent } from '$lib/server/mirror-from-event';
import { MirrorNotFoundError } from '$lib/server/encrypted-mirror';

export const GET: RequestHandler = async (event) => {
	const user = requireUser(event);
	const mirror = mirrorFromEvent(event);
	try {
		const text = await mirror.getRecoveryPackage(user.id, event.params.packageId);
		return new Response(text, { headers: { 'Content-Type': 'application/eisen-recovery' } });
	} catch (e) {
		if (e instanceof MirrorNotFoundError) throw error(404, 'Not found');
		throw e;
	}
};
