import { error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ params, url, platform }) => {
	const d1 = platform?.env?.DB;
	const r2: any = platform?.env?.ATTACHMENTS;
	if (!d1) throw error(500, 'D1 binding not configured.');
	if (!r2) throw error(500, 'R2 binding not configured.');

	const packageId = params.packageId;
	const ownerId = url.searchParams.get('ownerId');
	const deviceId = url.searchParams.get('deviceId');
	if (!packageId || !ownerId || !deviceId) throw error(400, 'Missing fields.');

	const device = await d1
		.prepare('SELECT device_id FROM devices WHERE owner_id = ? AND device_id = ? AND revoked_at IS NULL')
		.bind(ownerId, deviceId)
		.first();

	if (!device) throw error(403, 'Device is not enrolled.');

	const row = await d1
		.prepare('SELECT r2_key FROM backups WHERE package_id = ? AND owner_id = ?')
		.bind(packageId, ownerId)
		.first<{ r2_key: string }>();

	if (!row) throw error(404, 'Backup not found.');

	const object = await r2.get(row.r2_key);
	if (!object) throw error(404, 'Backup object not found.');

	const text = await object.text();
	return new Response(text, {
		headers: { 'Content-Type': 'application/eisen-recovery' }
	});
};
