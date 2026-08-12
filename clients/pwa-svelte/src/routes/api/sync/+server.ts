import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

type ClientRecord = {
	recordId: string;
	encryptedBlob: string;
	modifiedAt: number;
	deleted: number;
};

type SyncPayload = {
	ownerId: string;
	deviceId: string;
	lastVersion: number;
	changes: ClientRecord[];
};

export const POST: RequestHandler = async ({ request, platform }) => {
	if (!platform?.env?.DB) {
		throw error(500, 'D1 binding not configured');
	}
	const d1 = platform.env.DB;

	const { ownerId, deviceId, lastVersion, changes } = (await request.json()) as SyncPayload;

	if (!ownerId || !deviceId) {
		throw error(400, 'Missing owner or device id.');
	}

	const device = await d1
		.prepare('SELECT device_id FROM devices WHERE owner_id = ? AND device_id = ? AND revoked_at IS NULL')
		.bind(ownerId, deviceId)
		.first();

	if (!device) {
		throw error(403, 'Device is not enrolled for this account.');
	}

	for (const change of changes) {
		const row = await d1
			.prepare('SELECT IFNULL(MAX(sync_version), 0) + 1 AS v FROM vault_records WHERE owner_id = ?')
			.bind(ownerId)
			.first<number>('v');
		const nextVersion = row ?? 1;

		await d1
			.prepare(
				`
				INSERT INTO vault_records (record_id, owner_id, encrypted_blob, modified_at, sync_version, deleted)
				VALUES (?, ?, ?, ?, ?, ?)
				ON CONFLICT(record_id) DO UPDATE SET
					owner_id = excluded.owner_id,
					encrypted_blob = excluded.encrypted_blob,
					modified_at = excluded.modified_at,
					sync_version = excluded.sync_version,
					deleted = excluded.deleted
				`
			)
			.bind(change.recordId, ownerId, change.encryptedBlob, change.modifiedAt, nextVersion, change.deleted)
			.run();
	}

	await d1
		.prepare('UPDATE devices SET last_seen_at = ? WHERE device_id = ?')
		.bind(Date.now(), deviceId)
		.run();

	const { results } = await d1
		.prepare(
			'SELECT record_id AS recordId, encrypted_blob AS encryptedBlob, modified_at AS modifiedAt, sync_version AS syncVersion, deleted FROM vault_records WHERE owner_id = ? AND sync_version > ? ORDER BY sync_version'
		)
		.bind(ownerId, lastVersion)
		.all();

	const maxRow = await d1
		.prepare('SELECT IFNULL(MAX(sync_version), 0) AS v FROM vault_records WHERE owner_id = ?')
		.bind(ownerId)
		.first<number>('v');
	const last = maxRow ?? 0;

	return json({ changes: results, lastVersion: last });
};
