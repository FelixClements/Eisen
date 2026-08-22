import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { requireUser } from '$lib/server/require-user';
import { mirrorFromEvent } from '$lib/server/mirror-from-event';

export const POST: RequestHandler = async (event) => {
	const user = requireUser(event);
	if (!event.platform?.env?.ATTACHMENTS) throw error(500, 'R2 binding not configured');
	const mirror = mirrorFromEvent(event);
	const { packageId, packageText } = (await event.request.json()) as {
		packageId: string;
		packageText: string;
	};
	if (!packageId || !packageText) throw error(400, 'Missing backup fields.');
	await mirror.storeRecoveryPackage(user.id, packageId, packageText);
	return json({ success: true, packageId });
};

export const GET: RequestHandler = async (event) => {
	const user = requireUser(event);
	const mirror = mirrorFromEvent(event);
	const backups = await mirror.listRecoveryPackages(user.id);
	return json({
		backups: backups.map((b) => ({ packageId: b.id, createdAt: b.createdAt }))
	});
};
