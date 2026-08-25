import 'fake-indexeddb/auto';
import { describe, expect, it } from 'vitest';
import { createEncryptedMirror } from '$lib/server/encrypted-mirror';
import {
	memoryMirrorDatabase,
	memoryRecoveryObjects,
	recordingPushDispatch
} from '$lib/server/adapters/memory';
import { cloudPortFor } from './memory-cloud';
import { openWorkspace, requireOpen } from './workspace';
import { vaultKeyFromPassword } from './vault-key';

const ITER = 1;
const ACCOUNT = { id: 'acct-1' };
const PASSWORD = 'correct horse';

function newMirror() {
	return createEncryptedMirror({
		database: memoryMirrorDatabase(),
		recoveryObjects: memoryRecoveryObjects(),
		pushDispatch: recordingPushDispatch()
	});
}

function clock(start = 1_000) {
	let t = start;
	let n = 0;
	return {
		now: () => t,
		advance: (ms: number) => {
			t += ms;
		},
		uuid: () => {
			n += 1;
			return `00000000-0000-4000-8000-${String(n).padStart(12, '0')}`;
		}
	};
}

describe('RecoveryService', () => {
	it('exports and imports with recovery passphrase', async () => {
		const mirror = newMirror();
		const cloud = cloudPortFor(mirror, ACCOUNT.id);
		const key = await vaultKeyFromPassword({ password: PASSWORD, cloud, iterations: ITER });
		const ws = openWorkspace({
			account: ACCOUNT,
			vaultKey: key,
			adapters: { cloud, clock: clock(), dbName: 'recovery-export', kdfIterations: ITER }
		});
		await ws.ready;
		await requireOpen(ws.state).apply({
			kind: 'create',
			title: 'Recover me',
			important: true,
			urgent: true
		});
		const packed = await requireOpen(ws.state).recovery.export('recovery-passphrase');
		expect(packed.ok).toBe(true);
		ws.close();

		const ws2 = openWorkspace({
			account: ACCOUNT,
			vaultKey: key,
			adapters: { cloud, clock: clock(2000), dbName: 'recovery-import', kdfIterations: ITER }
		});
		await ws2.ready;
		if (!packed.ok) throw new Error('export failed');
		const text = await packed.value.text();
		const file = new File([text], 'recovery.json', { type: 'application/json' });
		const imported = await requireOpen(ws2.state).recovery.import(file, 'recovery-passphrase');
		expect(imported.ok).toBe(true);
		expect(requireOpen(ws2.state).matrix.cells['do-now'].tasks.map((t) => t.title)).toEqual([
			'Recover me'
		]);
		ws2.close();
	});

	it('rejects wrong recovery passphrase', async () => {
		const mirror = newMirror();
		const cloud = cloudPortFor(mirror, ACCOUNT.id);
		const key = await vaultKeyFromPassword({ password: PASSWORD, cloud, iterations: ITER });
		const ws = openWorkspace({
			account: ACCOUNT,
			vaultKey: key,
			adapters: { cloud, clock: clock(), dbName: 'recovery-wrong', kdfIterations: ITER }
		});
		await ws.ready;
		await requireOpen(ws.state).apply({
			kind: 'create',
			title: 'Secret',
			important: true,
			urgent: true
		});
		const packed = await requireOpen(ws.state).recovery.export('recovery-passphrase');
		ws.close();
		if (!packed.ok) throw new Error('export failed');
		const text = await packed.value.text();
		const file = new File([text], 'recovery.json');
		const ws2 = openWorkspace({
			account: ACCOUNT,
			vaultKey: key,
			adapters: { cloud, clock: clock(3000), dbName: 'recovery-wrong-2', kdfIterations: ITER }
		});
		await ws2.ready;
		const result = await requireOpen(ws2.state).recovery.import(file, 'wrong-passphrase');
		expect(result.ok).toBe(false);
		if (!result.ok) expect(result.error.code).toBe('wrong-passphrase');
		ws2.close();
	});
});
