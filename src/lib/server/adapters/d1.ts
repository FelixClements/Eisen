import type {
	BackupMetaRow,
	DueWake,
	MirrorDatabasePort,
	PushSubscriptionRow,
	VaultRecordRow
} from '../encrypted-mirror';
import { PushSubscriptionConflictError } from '../encrypted-mirror';
import type { VaultParams } from '$lib/workspace/ports';

export function d1MirrorDatabase(d1: D1Database): MirrorDatabasePort {
	return {
		async getVaultParams(accountId) {
			const row = await d1
				.prepare(
					'SELECT kdf_salt AS salt, check_blob AS checkBlob FROM vault_params WHERE user_id = ?'
				)
				.bind(accountId)
				.first<VaultParams>();
			return row ?? null;
		},
		async insertVaultParams(accountId, params) {
			try {
				await d1
					.prepare(
						`INSERT INTO vault_params (user_id, kdf_salt, check_blob, created_at)
						 VALUES (?, ?, ?, ?)`
					)
					.bind(accountId, params.salt, params.checkBlob, Date.now())
					.run();
				return 'ok';
			} catch {
				const existing = await this.getVaultParams(accountId);
				if (existing) return 'exists';
				throw new Error('Failed to insert vault params.');
			}
		},
		async nextSyncVersion(accountId) {
			const row = await d1
				.prepare('SELECT IFNULL(MAX(sync_version), 0) + 1 AS v FROM vault_records WHERE user_id = ?')
				.bind(accountId)
				.first<{ v: number }>();
			return row?.v ?? 1;
		},
		async getRecord(accountId, recordId) {
			return (
				(await d1
					.prepare(
						`SELECT record_id AS recordId, encrypted_blob AS encryptedBlob, modified_at AS modifiedAt,
						        device_id AS deviceId, sync_version AS syncVersion, deleted
						 FROM vault_records WHERE user_id = ? AND record_id = ?`
					)
					.bind(accountId, recordId)
					.first<VaultRecordRow>()) ?? null
			);
		},
		async upsertRecord(accountId, record: VaultRecordRow) {
			await d1
				.prepare(
					`INSERT INTO vault_records (record_id, user_id, encrypted_blob, modified_at, device_id, sync_version, deleted)
					 VALUES (?, ?, ?, ?, ?, ?, ?)
					 ON CONFLICT(record_id) DO UPDATE SET
					   user_id = excluded.user_id,
					   encrypted_blob = excluded.encrypted_blob,
					   modified_at = excluded.modified_at,
					   device_id = excluded.device_id,
					   sync_version = excluded.sync_version,
					   deleted = excluded.deleted`
				)
				.bind(
					record.recordId,
					accountId,
					record.encryptedBlob,
					record.modifiedAt,
					record.deviceId,
					record.syncVersion,
					record.deleted
				)
				.run();
		},
		async recordsAfter(accountId, lastVersion) {
			const { results } = await d1
				.prepare(
					`SELECT record_id AS recordId, encrypted_blob AS encryptedBlob, modified_at AS modifiedAt,
					        device_id AS deviceId, sync_version AS syncVersion, deleted
					 FROM vault_records WHERE user_id = ? AND sync_version > ? ORDER BY sync_version`
				)
				.bind(accountId, lastVersion)
				.all<VaultRecordRow>();
			return results ?? [];
		},
		async maxSyncVersion(accountId) {
			const row = await d1
				.prepare('SELECT IFNULL(MAX(sync_version), 0) AS v FROM vault_records WHERE user_id = ?')
				.bind(accountId)
				.first<{ v: number }>();
			return row?.v ?? 0;
		},
		async insertBackupMeta(accountId, row: BackupMetaRow) {
			await d1
				.prepare(
					`INSERT INTO backups (package_id, user_id, r2_key, created_at)
					 VALUES (?, ?, ?, ?)
					 ON CONFLICT(package_id) DO UPDATE SET r2_key = excluded.r2_key, created_at = excluded.created_at`
				)
				.bind(row.packageId, accountId, row.r2Key, row.createdAt)
				.run();
		},
		async listBackupMeta(accountId) {
			const { results } = await d1
				.prepare(
					`SELECT package_id AS packageId, r2_key AS r2Key, created_at AS createdAt
					 FROM backups WHERE user_id = ? ORDER BY created_at DESC`
				)
				.bind(accountId)
				.all<BackupMetaRow>();
			return results ?? [];
		},
		async getBackupMeta(accountId, packageId) {
			return (
				(await d1
					.prepare(
						`SELECT package_id AS packageId, r2_key AS r2Key, created_at AS createdAt
						 FROM backups WHERE user_id = ? AND package_id = ?`
					)
					.bind(accountId, packageId)
					.first<BackupMetaRow>()) ?? null
			);
		},
		async upsertPushSubscription(accountId, sub: PushSubscriptionRow) {
			const existing = await d1
				.prepare('SELECT user_id AS userId FROM push_subscriptions WHERE endpoint = ?')
				.bind(sub.endpoint)
				.first<{ userId: string }>();
			if (existing && existing.userId !== accountId) {
				throw new PushSubscriptionConflictError();
			}
			await d1
				.prepare(
					`INSERT INTO push_subscriptions (id, user_id, device_id, endpoint, p256dh, auth, created_at)
					 VALUES (?, ?, ?, ?, ?, ?, ?)
					 ON CONFLICT(endpoint) DO UPDATE SET
					   user_id = excluded.user_id,
					   device_id = excluded.device_id,
					   p256dh = excluded.p256dh,
					   auth = excluded.auth`
				)
				.bind(
					crypto.randomUUID(),
					accountId,
					sub.deviceId,
					sub.endpoint,
					sub.p256dh,
					sub.auth,
					Date.now()
				)
				.run();
		},
		async deletePushSubscription(endpoint) {
			await d1.prepare('DELETE FROM push_subscriptions WHERE endpoint = ?').bind(endpoint).run();
		},
		async getPushSubscription(accountId, deviceId) {
			return (
				(await d1
					.prepare(
						`SELECT device_id AS deviceId, endpoint, p256dh, auth
						 FROM push_subscriptions
						 WHERE user_id = ? AND device_id = ?`
					)
					.bind(accountId, deviceId)
					.first<PushSubscriptionRow>()) ?? null
			);
		},
		async insertWake(accountId, id, schedule) {
			await d1
				.prepare(
					`DELETE FROM wake_schedules
					 WHERE user_id = ? AND device_id = ? AND sent = 0`
				)
				.bind(accountId, schedule.deviceId)
				.run();
			await d1
				.prepare(
					`INSERT INTO wake_schedules (id, user_id, device_id, wake_at, nonce, sent)
					 VALUES (?, ?, ?, ?, ?, 0)`
				)
				.bind(id, accountId, schedule.deviceId, schedule.wakeAt, schedule.nonce)
				.run();
		},
		async dueWakes(now) {
			const { results } = await d1
				.prepare(
					`SELECT ws.id, ws.user_id AS userId, ws.device_id AS deviceId,
					        ps.endpoint, ps.p256dh, ps.auth
					 FROM wake_schedules ws
					 JOIN push_subscriptions ps
					   ON ps.user_id = ws.user_id AND ps.device_id = ws.device_id
					 WHERE ws.sent = 0 AND ws.wake_at <= ?`
				)
				.bind(now)
				.all<DueWake>();
			return results ?? [];
		},
		async markWakeSent(id) {
			await d1.prepare('UPDATE wake_schedules SET sent = 1 WHERE id = ?').bind(id).run();
		},
		async incrementWakeAttempts(id) {
			await d1
				.prepare('UPDATE wake_schedules SET attempts = attempts + 1 WHERE id = ?')
				.bind(id)
				.run();
			const row = await d1
				.prepare('SELECT attempts FROM wake_schedules WHERE id = ?')
				.bind(id)
				.first<{ attempts: number }>();
			return row?.attempts ?? 0;
		}
	};
}
