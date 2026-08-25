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
import type { VaultParams } from '$lib/workspace/ports';

export function memoryMirrorDatabase(): MirrorDatabasePort {
	const params = new Map<string, VaultParams>();
	const records = new Map<string, Map<string, VaultRecordRow>>();
	const backups = new Map<string, BackupMetaRow[]>();
	const pushes = new Map<string, PushSubscriptionRow[]>();
	const wakes: Array<
		WakeScheduleRow & { id: string; userId: string; sent: boolean }
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
			const list = (pushes.get(accountId) ?? []).filter((s) => s.endpoint !== sub.endpoint);
			list.push(sub);
			pushes.set(accountId, list);
		},
		async insertWake(accountId, id, schedule) {
			wakes.push({ ...schedule, id, userId: accountId, sent: false });
		},
		async dueWakes(now) {
			const due: DueWake[] = [];
			for (const w of wakes) {
				if (w.sent || w.wakeAt > now) continue;
				const subs = pushes.get(w.userId) ?? [];
				for (const sub of subs) {
					due.push({
						id: w.id,
						userId: w.userId,
						endpoint: sub.endpoint,
						p256dh: sub.p256dh,
						auth: sub.auth
					});
				}
			}
			return due;
		},
		async markWakeSent(id) {
			const w = wakes.find((x) => x.id === id);
			if (w) w.sent = true;
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
