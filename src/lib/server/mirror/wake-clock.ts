import type {
	MirrorDatabasePort,
	PushDispatchPort,
	PushSubscriptionRow,
	WakeScheduleRow
} from './types';
import { PushSubscriptionNotFoundError } from './types';

export function createWakeClockApi(database: MirrorDatabasePort, pushDispatch: PushDispatchPort) {
	return {
		async registerPushSubscription(accountId: string, sub: PushSubscriptionRow) {
			await database.upsertPushSubscription(accountId, sub);
		},
		async unregisterPushSubscription(accountId: string, deviceId: string) {
			await database.deletePushSubscriptionForDevice(accountId, deviceId);
		},
		async sendTestPush(accountId: string, deviceId: string) {
			const sub = await database.getPushSubscription(accountId, deviceId);
			if (!sub) throw new PushSubscriptionNotFoundError();
			await pushDispatch.send(sub, JSON.stringify({ type: 'test' }));
		},
		async scheduleWake(accountId: string, schedule: WakeScheduleRow) {
			const scheduleId = crypto.randomUUID();
			await database.insertWake(accountId, scheduleId, schedule);
			return { scheduleId };
		},
		async dispatchDueWakes(now: number) {
			const due = await database.dueWakes(now);
			let sent = 0;
			let failed = 0;
			for (const row of due) {
				try {
					await pushDispatch.send(
						{
							deviceId: row.deviceId,
							endpoint: row.endpoint,
							p256dh: row.p256dh,
							auth: row.auth
						},
						JSON.stringify({ type: 'wake' })
					);
					await database.markWakeSent(row.id);
					sent++;
				} catch (err) {
					failed++;
					const status = err && typeof err === 'object' && 'status' in err ? Number(err.status) : 0;
					const host = (() => {
						try {
							return new URL(row.endpoint).host;
						} catch {
							return 'unknown';
						}
					})();
					console.error(`push dispatch failed host=${host} status=${status || 'unknown'}`);
					if (status === 404 || status === 410) {
						await database.deletePushSubscription(row.endpoint);
						await database.markWakeSent(row.id);
						continue;
					}
					const attempts = await database.incrementWakeAttempts(row.id);
					if (attempts >= 5) {
						await database.markWakeSent(row.id);
					}
				}
			}
			return { sent, failed };
		}
	};
}
