import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, platform }) => {
	const d1 = platform?.env?.DB;
	const r2: any = platform?.env?.ATTACHMENTS;
	if (!d1) throw error(500, 'D1 binding not configured.');
	if (!r2) throw error(500, 'R2 binding not configured.');

	const { ownerId, deviceId, packageId, packageText } = (await request.json()) as {
		ownerId: string;
		deviceId: string;
		packageId: string;
		packageText: string;
	};

	if (!ownerId || !deviceId || !packageId || !packageText) {
		throw error(400, 'Missing backup fields.');
	}

	const device = await d1
		.prepare('SELECT device_id FROM devices WHERE owner_id = ? AND device_id = ? AND revoked_at IS NULL')
		.bind(ownerId, deviceId)
		.first();

	if (!device) {
		throw error(403, 'Device is not enrolled for this account.');
	}

	const r2Key = `backups/${ownerId}/${packageId}`;
	await r2.put(r2Key, packageText, {
		httpMetadata: { contentType: 'application/eisen-recovery' }
	});

	await d1
		.prepare(
			`INSERT INTO backups (package_id, owner_id, r2_key, created_at)
			 VALUES (?, ?, ?, ?)
			 ON CONFLICT(package_id) DO UPDATE SET
			   r2_key = excluded.r2_key,
			   created_at = excluded.created_at`
		)
		.bind(packageId, ownerId, r2Key, Date.now())
		.run();

	return json({ success: true, packageId });
};

export const GET: RequestHandler = async ({ url, platform }) => {
	const d1 = platform?.env?.DB;
	if (!d1) throw error(500, 'D1 binding not configured.');

	const ownerId = url.searchParams.get('ownerId');
	const deviceId = url.searchParams.get('deviceId');
	if (!ownerId || !deviceId) throw error(400, 'Missing owner or device id.');

	const device = await d1
		.prepare('SELECT device_id FROM devices WHERE owner_id = ? AND device_id = ? AND revoked_at IS NULL')
		.bind(ownerId, deviceId)
		.first();

	if (!device) throw error(403, 'Device is not enrolled.');

	const { results } = await d1
		.prepare('SELECT package_id AS packageId, created_at AS createdAt FROM backups WHERE owner_id = ? ORDER BY created_at DESC')
		.bind(ownerId)
		.all();

	return json({ backups: results });
};
