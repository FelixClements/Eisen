import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, platform }) => {
	const kv = platform?.env?.KV;
	const d1 = platform?.env?.DB;
	if (!kv || !d1) {
		throw error(500, 'KV or D1 binding not configured.');
	}

	const { code, deviceId } = (await request.json()) as { code: string; deviceId: string };

	if (!code || !deviceId) {
		throw error(400, 'Missing pairing code or device id.');
	}

	const stored = await kv.get(`pairing:${code}`);
	if (!stored) {
		throw error(400, 'Invalid or expired pairing code.');
	}

	const { ownerId, vaultId, expiresAt } = JSON.parse(stored) as {
		ownerId: string;
		vaultId: string;
		expiresAt: number;
	};

	if (Date.now() > expiresAt) {
		await kv.delete(`pairing:${code}`);
		throw error(400, 'Pairing code expired.');
	}

	await kv.delete(`pairing:${code}`);

	await d1
		.prepare(
			`INSERT OR IGNORE INTO accounts (owner_id, vault_id, created_at, device_count)
			 VALUES (?, ?, ?, ?)`
		)
		.bind(ownerId, vaultId, Date.now(), 1)
		.run();

	await d1
		.prepare(
			`INSERT OR IGNORE INTO devices (device_id, owner_id, enrolled_at, last_seen_at)
			 VALUES (?, ?, ?, ?)`
		)
		.bind(deviceId, ownerId, Date.now(), Date.now())
		.run();

	return json({ ownerId, vaultId });
};
