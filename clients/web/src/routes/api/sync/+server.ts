import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { requireUser } from '$lib/server/require-user';

type ClientRecord = {
	recordId: string;
	encryptedBlob: string;
	modifiedAt: number;
	deleted: number;
};

export const POST: RequestHandler = async (event) => {
	const user = requireUser(event);
	const d1 = event.platform?.env?.DB;
	if (!d1) throw error(500, 'D1 binding not configured');

	const { lastVersion, changes } = (await event.request.json()) as {
		deviceId?: string;
		lastVersion: number;
		changes: ClientRecord[];
	};

	const userId = user.id;

	for (const change of changes) {
		const row = await d1
			.prepare('SELECT IFNULL(MAX(sync_version), 0) + 1 AS v FROM vault_records WHERE user_id = ?')
			.bind(userId)
			.first<{ v: number }>();
		const nextVersion = row?.v ?? 1;

		await d1
			.prepare(
				`INSERT INTO vault_records (record_id, user_id, encrypted_blob, modified_at, sync_version, deleted)
				 VALUES (?, ?, ?, ?, ?, ?)
				 ON CONFLICT(record_id) DO UPDATE SET
				   user_id = excluded.user_id,
				   encrypted_blob = excluded.encrypted_blob,
				   modified_at = excluded.modified_at,
				   sync_version = excluded.sync_version,
				   deleted = excluded.deleted`
			)
			.bind(change.recordId, userId, change.encryptedBlob, change.modifiedAt, nextVersion, change.deleted)
			.run();
	}

	const { results } = await d1
		.prepare(
			`SELECT record_id AS recordId, encrypted_blob AS encryptedBlob, modified_at AS modifiedAt,
			        sync_version AS syncVersion, deleted
			 FROM vault_records WHERE user_id = ? AND sync_version > ? ORDER BY sync_version`
		)
		.bind(userId, lastVersion ?? 0)
		.all();

	const maxRow = await d1
		.prepare('SELECT IFNULL(MAX(sync_version), 0) AS v FROM vault_records WHERE user_id = ?')
		.bind(userId)
		.first<{ v: number }>();

	return json({ changes: results, lastVersion: maxRow?.v ?? 0 });
};
