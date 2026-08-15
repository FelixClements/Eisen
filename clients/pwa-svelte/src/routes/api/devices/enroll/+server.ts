import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, platform }) => {
	const d1 = platform?.env?.DB;
	if (!d1) {
		throw error(500, 'D1 binding not configured.');
	}

	const { ownerId, vaultId, deviceId } = (await request.json()) as {
		ownerId: string;
		vaultId: string;
		deviceId: string;
	};

	if (!ownerId || !vaultId || !deviceId) {
		throw error(400, 'Missing device fields.');
	}

	await d1
		.prepare(
			`INSERT INTO accounts (owner_id, vault_id, created_at, last_sync_at, device_count)
			 VALUES (?, ?, ?, ?, ?)
			 ON CONFLICT(owner_id) DO UPDATE SET
			   vault_id = excluded.vault_id,
			   last_sync_at = excluded.last_sync_at,
			   device_count = excluded.device_count`
		)
		.bind(ownerId, vaultId, Date.now(), null, 1)
		.run();

	await d1
		.prepare(
			`INSERT OR IGNORE INTO devices (device_id, owner_id, enrolled_at, last_seen_at, revoked_at)
			 VALUES (?, ?, ?, ?, ?)`
		)
		.bind(deviceId, ownerId, Date.now(), Date.now(), null)
		.run();

	return json({ success: true });
};
