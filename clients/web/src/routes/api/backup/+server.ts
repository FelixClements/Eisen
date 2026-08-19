import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { requireUser } from '$lib/server/require-user';

export const POST: RequestHandler = async (event) => {
	const user = requireUser(event);
	const d1 = event.platform?.env?.DB;
	const r2 = event.platform?.env?.ATTACHMENTS;
	if (!d1) throw error(500, 'D1 binding not configured');
	if (!r2) throw error(500, 'R2 binding not configured');

	const { packageId, packageText } = (await event.request.json()) as {
		packageId: string;
		packageText: string;
	};

	if (!packageId || !packageText) throw error(400, 'Missing backup fields.');

	const userId = user.id;
	const r2Key = `backups/${userId}/${packageId}`;
	await r2.put(r2Key, packageText, {
		httpMetadata: { contentType: 'application/eisen-recovery' }
	});

	await d1
		.prepare(
			`INSERT INTO backups (package_id, user_id, r2_key, created_at)
			 VALUES (?, ?, ?, ?)
			 ON CONFLICT(package_id) DO UPDATE SET r2_key = excluded.r2_key, created_at = excluded.created_at`
		)
		.bind(packageId, userId, r2Key, Date.now())
		.run();

	return json({ success: true, packageId });
};

export const GET: RequestHandler = async (event) => {
	const user = requireUser(event);
	const d1 = event.platform?.env?.DB;
	if (!d1) throw error(500, 'D1 binding not configured');

	const { results } = await d1
		.prepare(
			'SELECT package_id AS packageId, created_at AS createdAt FROM backups WHERE user_id = ? ORDER BY created_at DESC'
		)
		.bind(user.id)
		.all();

	return json({ backups: results });
};
