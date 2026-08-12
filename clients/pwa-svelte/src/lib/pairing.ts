import { browser } from '$app/environment';
import { db, getAccount } from './db';

const CHARS = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';

function generateShortCode(): string {
	let code = '';
	const arr = new Uint8Array(6);
	crypto.getRandomValues(arr);
	for (let i = 0; i < 6; i++) {
		code += CHARS[arr[i] % CHARS.length];
	}
	return code;
}

export interface PairingCode {
	code: string;
	expiresAt: number;
}

export async function initiatePairing(): Promise<PairingCode> {
	if (!browser) throw new Error('Pairing is browser-only.');

	const account = await getAccount();
	if (!account) throw new Error('No account to pair.');

	const device = await db.deviceState.toCollection().first();
	if (!device) throw new Error('No device state.');

	const code = generateShortCode();
	const expiresAt = Date.now() + 5 * 60 * 1000;

	const res = await fetch('/api/pairing/initiate', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({
			code,
			ownerId: account.ownerId,
			vaultId: account.vaultId,
			deviceId: device.deviceId,
			expiresAt
		})
	});

	if (!res.ok) {
		throw new Error('Failed to initiate pairing.');
	}

	return { code, expiresAt };
}

export async function claimPairingCode(code: string): Promise<{ ownerId: string; vaultId: string }> {
	if (!browser) throw new Error('Pairing is browser-only.');

	const deviceId = crypto.randomUUID();
	const res = await fetch('/api/pairing/claim', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ code, deviceId })
	});

	if (!res.ok) {
		throw new Error('Invalid or expired pairing code.');
	}

	const { ownerId, vaultId } = (await res.json()) as { ownerId: string; vaultId: string };

	await db.transaction('rw', db.accounts, db.deviceState, async () => {
		await db.accounts.clear();
		await db.deviceState.clear();
		await db.accounts.add({
			ownerId,
			vaultId,
			deviceSalt: '',
			validationValue: '',
			createdAt: Date.now()
		});
		await db.deviceState.add({ deviceId, ownerId, lastSyncAt: null });
	});

	return { ownerId, vaultId };
}
