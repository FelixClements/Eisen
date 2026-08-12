import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, platform }) => {
	const kv = platform?.env?.KV;
	const d1 = platform?.env?.DB;
	if (!kv || !d1) {
		throw error(500, 'KV or D1 binding not configured.');
	}

	const { code, ownerId, vaultId, deviceId, expiresAt } = (await request.json()) as {
		code: string;
		ownerId: string;
		vaultId: string;
		deviceId: string;
		expiresAt: number;
	};

	if (!code || !ownerId || !vaultId || !deviceId || !expiresAt) {
		throw error(400, 'Missing pairing fields.');
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
			`INSERT INTO devices (device_id, owner_id, enrolled_at, last_seen_at, revoked_at)
			 VALUES (?, ?, ?, ?, ?)
			 ON CONFLICT(device_id) DO UPDATE SET
			   owner_id = excluded.owner_id,
			   enrolled_at = excluded.enrolled_at,
			   last_seen_at = excluded.last_seen_at`
		)
		.bind(deviceId, ownerId, Date.now(), Date.now(), null)
		.run();

	await kv.put(
		`pairing:${code}`,
		JSON.stringify({ ownerId, vaultId, expiresAt }),
		{ expirationTtl: 300 }
	);

	return json({ success: true });
};
