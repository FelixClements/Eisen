import { describe, expect, it } from 'vitest';
import { createEncryptedMirror, PushSubscriptionConflictError } from './encrypted-mirror';
import {
	memoryMirrorDatabase,
	memoryRecoveryObjects,
	recordingPushDispatch
} from './adapters/memory';
import { PushSendError } from './adapters/web-push';

const ACCOUNT = 'acct-1';
const DEVICE = '00000000-0000-4000-8000-000000000001';

function newMirror(pushDispatch = recordingPushDispatch()) {
	return createEncryptedMirror({
		database: memoryMirrorDatabase(),
		recoveryObjects: memoryRecoveryObjects(),
		pushDispatch
	});
}

describe('push wake dispatch', () => {
	it('dispatches due wakes to the matching device subscription', async () => {
		const push = recordingPushDispatch();
		const mirror = newMirror(push);
		await mirror.registerPushSubscription(ACCOUNT, {
			deviceId: DEVICE,
			endpoint: 'https://push.example/sub',
			p256dh: 'x'.repeat(87),
			auth: 'y'.repeat(22)
		});
		await mirror.scheduleWake(ACCOUNT, {
			deviceId: DEVICE,
			wakeAt: 1_000,
			nonce: 'n1'
		});
		const result = await mirror.dispatchDueWakes(2_000);
		expect(result).toEqual({ sent: 1, failed: 0 });
		expect(push.sent).toEqual([JSON.stringify({ type: 'wake' })]);
	});

	it('does not dispatch to a different device subscription', async () => {
		const push = recordingPushDispatch();
		const mirror = newMirror(push);
		await mirror.registerPushSubscription(ACCOUNT, {
			deviceId: '00000000-0000-4000-8000-000000000099',
			endpoint: 'https://push.example/other',
			p256dh: 'x'.repeat(87),
			auth: 'y'.repeat(22)
		});
		await mirror.scheduleWake(ACCOUNT, {
			deviceId: DEVICE,
			wakeAt: 1_000,
			nonce: 'n1'
		});
		const result = await mirror.dispatchDueWakes(2_000);
		expect(result).toEqual({ sent: 0, failed: 0 });
		expect(push.sent).toEqual([]);
	});

	it('replaces prior unsent wakes for the same device', async () => {
		const push = recordingPushDispatch();
		const mirror = newMirror(push);
		await mirror.registerPushSubscription(ACCOUNT, {
			deviceId: DEVICE,
			endpoint: 'https://fcm.googleapis.com/fcm/send/test',
			p256dh: 'x'.repeat(87),
			auth: 'y'.repeat(22)
		});
		await mirror.scheduleWake(ACCOUNT, { deviceId: DEVICE, wakeAt: 5_000, nonce: 'a' });
		await mirror.scheduleWake(ACCOUNT, { deviceId: DEVICE, wakeAt: 9_000, nonce: 'b' });
		const before = await mirror.dispatchDueWakes(8_000);
		expect(before).toEqual({ sent: 0, failed: 0 });
		const after = await mirror.dispatchDueWakes(10_000);
		expect(after).toEqual({ sent: 1, failed: 0 });
		expect(push.sent).toHaveLength(1);
	});

	it('rejects cross-account endpoint takeover', async () => {
		const mirror = newMirror();
		await mirror.registerPushSubscription(ACCOUNT, {
			deviceId: DEVICE,
			endpoint: 'https://fcm.googleapis.com/fcm/send/shared',
			p256dh: 'x'.repeat(87),
			auth: 'y'.repeat(22)
		});
		await expect(
			mirror.registerPushSubscription('acct-2', {
				deviceId: '00000000-0000-4000-8000-000000000002',
				endpoint: 'https://fcm.googleapis.com/fcm/send/shared',
				p256dh: 'y'.repeat(87),
				auth: 'z'.repeat(22)
			})
		).rejects.toBeInstanceOf(PushSubscriptionConflictError);
	});

	it('deletes stale subscriptions on 410', async () => {
		const db = memoryMirrorDatabase();
		const mirror = createEncryptedMirror({
			database: db,
			recoveryObjects: memoryRecoveryObjects(),
			pushDispatch: {
				async send() {
					throw new PushSendError('gone', 410);
				}
			}
		});
		await mirror.registerPushSubscription(ACCOUNT, {
			deviceId: DEVICE,
			endpoint: 'https://push.example/sub',
			p256dh: 'x'.repeat(87),
			auth: 'y'.repeat(22)
		});
		await mirror.scheduleWake(ACCOUNT, { deviceId: DEVICE, wakeAt: 1_000, nonce: 'n1' });
		const first = await mirror.dispatchDueWakes(2_000);
		expect(first.failed).toBe(1);
		const second = await mirror.dispatchDueWakes(2_000);
		expect(second).toEqual({ sent: 0, failed: 0 });
	});
});
