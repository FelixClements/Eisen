import type {
	BackupMetaRow,
	DueWake,
	MirrorDatabasePort,
	PushDispatchPort,
	PushSubscriptionRow,
	RecoveryObjectPort,
	VaultRecordRow,
	WakeScheduleRow
} from '../encrypted-mirror';
import { PushSubscriptionConflictError } from '../encrypted-mirror';
import { applyRecordLww } from '$lib/sync/record-lww';
import type { VaultParams } from '$lib/sync/types';

export function memoryMirrorDatabase(): MirrorDatabasePort {
	const params = new Map<string, VaultParams>();
	const records = new Map<string, Map<string, VaultRecordRow>>();
	const backups = new Map<string, BackupMetaRow[]>();
	const clocks = new Map<string, number>();
	const pushes = new Map<string, PushSubscriptionRow[]>();
	const wakes: Array<
		WakeScheduleRow & { id: string; userId: string; sent: boolean; attempts: number }
	> = [];

	function accountRecords(accountId: string): Map<string, VaultRecordRow> {
		let map = records.get(accountId);
		if (!map) {
			map = new Map();
			records.set(accountId, map);
		}
		return map;
	}

	return {
		async getVaultParams(accountId) {
			return params.get(accountId) ?? null;
		},
		async insertVaultParams(accountId, row) {
			if (params.has(accountId)) return 'exists';
			params.set(accountId, row);
			return 'ok';
		},
		async nextSyncVersion(accountId) {
			let max = 0;
			for (const rec of accountRecords(accountId).values()) {
				if (rec.syncVersion > max) max = rec.syncVersion;
			}
			return max + 1;
		},
		async upsertRecord(accountId, record) {
			accountRecords(accountId).set(record.recordId, record);
		},
		async applyLwwUpsert(accountId, incoming) {
			const existing = await this.getRecord(accountId, incoming.recordId);
			const decision = applyRecordLww(
				existing ? { modifiedAt: existing.modifiedAt, deviceId: existing.deviceId } : null,
				{ modifiedAt: incoming.modifiedAt, deviceId: incoming.deviceId }
			);
			if (decision === 'reject') return 'reject';
			const nextVersion = (clocks.get(accountId) ?? 0) + 1;
			clocks.set(accountId, nextVersion);
			accountRecords(accountId).set(incoming.recordId, {
				...incoming,
				syncVersion: nextVersion
			});
			return 'accept';
		},
		async getRecord(accountId, recordId) {
			return accountRecords(accountId).get(recordId) ?? null;
		},
		async recordsAfter(accountId, lastVersion) {
			return [...accountRecords(accountId).values()]
				.filter((r) => r.syncVersion > lastVersion)
				.sort((a, b) => a.syncVersion - b.syncVersion);
		},
		async maxSyncVersion(accountId) {
			let max = 0;
			for (const rec of accountRecords(accountId).values()) {
				if (rec.syncVersion > max) max = rec.syncVersion;
			}
			return max;
		},
		async insertBackupMeta(accountId, row) {
			const list = backups.get(accountId) ?? [];
			const next = list.filter((b) => b.packageId !== row.packageId);
			next.push(row);
			backups.set(accountId, next);
		},
		async listBackupMeta(accountId) {
			return [...(backups.get(accountId) ?? [])].sort((a, b) => b.createdAt - a.createdAt);
		},
		async getBackupMeta(accountId, packageId) {
			return (backups.get(accountId) ?? []).find((b) => b.packageId === packageId) ?? null;
		},
		async upsertPushSubscription(accountId, sub) {
			for (const [ownerId, list] of pushes) {
				if (ownerId === accountId) continue;
				if (list.some((s) => s.endpoint === sub.endpoint)) {
					throw new PushSubscriptionConflictError();
				}
			}
			const list = (pushes.get(accountId) ?? []).filter((s) => s.endpoint !== sub.endpoint);
			list.push(sub);
			pushes.set(accountId, list);
		},
		async deletePushSubscription(endpoint) {
			for (const [accountId, list] of pushes) {
				const next = list.filter((s) => s.endpoint !== endpoint);
				if (next.length !== list.length) pushes.set(accountId, next);
			}
		},
		async deletePushSubscriptionForDevice(accountId, deviceId) {
			const list = pushes.get(accountId) ?? [];
			pushes.set(
				accountId,
				list.filter((s) => s.deviceId !== deviceId)
			);
		},
		async getPushSubscription(accountId, deviceId) {
			return (pushes.get(accountId) ?? []).find((s) => s.deviceId === deviceId) ?? null;
		},
		async insertWake(accountId, id, schedule) {
			for (let i = wakes.length - 1; i >= 0; i--) {
				const w = wakes[i];
				if (w.userId === accountId && w.deviceId === schedule.deviceId && !w.sent) {
					wakes.splice(i, 1);
				}
			}
			wakes.push({ ...schedule, id, userId: accountId, sent: false, attempts: 0 });
		},
		async dueWakes(now) {
			const due: DueWake[] = [];
			for (const w of wakes) {
				if (w.sent || w.wakeAt > now) continue;
				const sub = (pushes.get(w.userId) ?? []).find((s) => s.deviceId === w.deviceId);
				if (!sub) continue;
				due.push({
					id: w.id,
					userId: w.userId,
					deviceId: w.deviceId,
					endpoint: sub.endpoint,
					p256dh: sub.p256dh,
					auth: sub.auth
				});
			}
			return due;
		},
		async markWakeSent(id) {
			const w = wakes.find((x) => x.id === id);
			if (w) w.sent = true;
		},
		async incrementWakeAttempts(id) {
			const w = wakes.find((x) => x.id === id);
			if (!w) return 0;
			w.attempts += 1;
			return w.attempts;
		}
	};
}

export function memoryRecoveryObjects(): RecoveryObjectPort {
	const blobs = new Map<string, string>();
	return {
		async put(key, body) {
			blobs.set(key, body);
		},
		async get(key) {
			return blobs.get(key) ?? null;
		}
	};
}

export function recordingPushDispatch(): PushDispatchPort & { sent: string[] } {
	const sent: string[] = [];
	return {
		sent,
		async send(_sub, payload) {
			sent.push(payload);
		}
	};
}
