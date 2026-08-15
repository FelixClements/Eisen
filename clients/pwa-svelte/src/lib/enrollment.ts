import { browser } from '$app/environment';
import { db, getAccount } from './db';
import type { Account } from './db';

export async function enrollDevice(account?: Account | undefined): Promise<void> {
	if (!browser) return;

	const a = account ?? (await getAccount());
	if (!a) throw new Error('No account to enroll.');

	const state = await db.deviceState.toCollection().first();
	if (!state) throw new Error('No device state.');

	const res = await fetch('/api/devices/enroll', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({
			ownerId: a.ownerId,
			vaultId: a.vaultId,
			deviceId: state.deviceId
		})
	});

	if (!res.ok) {
		throw new Error('Enrollment failed: ' + res.status);
	}
}
