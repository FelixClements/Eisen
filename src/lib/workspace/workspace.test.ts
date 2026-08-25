import 'fake-indexeddb/auto';
import { describe, expect, it } from 'vitest';
import { deriveAuthVerifier, deriveVaultKey } from '$lib/crypto';
import { createEncryptedMirror, VaultParamsExistError } from '$lib/server/encrypted-mirror';
import {
	memoryMirrorDatabase,
	memoryRecoveryObjects,
	recordingPushDispatch
} from '$lib/server/adapters/memory';
import { cloudPortFor } from './memory-cloud';
import { openWorkspace, requireOpen } from './workspace';
import { vaultKeyFromPassword } from './vault-key';
import type { SyncPushBatch } from './ports';

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

describe('Workspace', () => {
	it('creates a task into Do Now after sign-in derives a vault key', async () => {
		const mirror = newMirror();
		const cloud = cloudPortFor(mirror, ACCOUNT.id);
		const key = await vaultKeyFromPassword({ password: PASSWORD, cloud, iterations: ITER });
		const ws = openWorkspace({
			account: ACCOUNT,
			vaultKey: key,
			adapters: { cloud, clock: clock(), dbName: 'ws-create', kdfIterations: ITER }
		});
		await ws.ready;
		const open = requireOpen(ws.state);
		const id = await open.apply({
			kind: 'create',
			title: 'Buy milk',
			important: true,
			urgent: true
		});
		const matrix = requireOpen(ws.state).matrix;
		expect(matrix.cells['do-now'].tasks.map((t) => t.title)).toEqual(['Buy milk']);
		expect(id).toBeTruthy();
		ws.close();
	});

	it('completes a task into history', async () => {
		const mirror = newMirror();
		const cloud = cloudPortFor(mirror, ACCOUNT.id);
		const key = await vaultKeyFromPassword({ password: PASSWORD, cloud, iterations: ITER });
		const ws = openWorkspace({
			account: ACCOUNT,
			vaultKey: key,
			adapters: { cloud, clock: clock(), dbName: 'ws-complete', kdfIterations: ITER }
		});
		await ws.ready;
		const open = requireOpen(ws.state);
		const id = await open.apply({
			kind: 'create',
			title: 'Done me',
			important: true,
			urgent: false
		});
		await requireOpen(ws.state).apply({ kind: 'complete', id, done: true });
		const after = requireOpen(ws.state);
		expect(after.matrix.total).toBe(0);
		expect(after.history.completed.map((t) => t.title)).toEqual(['Done me']);
		ws.close();
	});

	it('lets a second device decrypt the first device tasks without a recovery import', async () => {
		const mirror = newMirror();
		const cloud = cloudPortFor(mirror, ACCOUNT.id);
		const time = clock();
		const keyA = await vaultKeyFromPassword({ password: PASSWORD, cloud, iterations: ITER });
		const a = openWorkspace({
			account: ACCOUNT,
			vaultKey: keyA,
			adapters: { cloud, clock: time, dbName: 'ws-dev-a', kdfIterations: ITER }
		});
		await a.ready;
		await requireOpen(a.state).apply({
			kind: 'create',
			title: 'Shared',
			important: true,
			urgent: true
		});
		await requireOpen(a.state).sync.now();

		const keyB = await vaultKeyFromPassword({ password: PASSWORD, cloud, iterations: ITER });
		const b = openWorkspace({
			account: ACCOUNT,
			vaultKey: keyB,
			adapters: { cloud, clock: clock(5_000), dbName: 'ws-dev-b', kdfIterations: ITER }
		});
		await b.ready;
		expect(requireOpen(b.state).matrix.cells['do-now'].tasks.map((t) => t.title)).toEqual(['Shared']);
		a.close();
		b.close();
	});

	it('does not let a stale remote clobber a newer local edit', async () => {
		const mirror = newMirror();
		const cloud = cloudPortFor(mirror, ACCOUNT.id);
		const timeA = clock(1_000);
		const key = await vaultKeyFromPassword({ password: PASSWORD, cloud, iterations: ITER });
		const a = openWorkspace({
			account: ACCOUNT,
			vaultKey: key,
			adapters: { cloud, clock: timeA, dbName: 'ws-lww-a', kdfIterations: ITER }
		});
		await a.ready;
		const id = await requireOpen(a.state).apply({
			kind: 'create',
			title: 'Original',
			important: false,
			urgent: false
		});
		await requireOpen(a.state).sync.now();

		const timeB = clock(2_000);
		const b = openWorkspace({
			account: ACCOUNT,
			vaultKey: key,
			adapters: { cloud, clock: timeB, dbName: 'ws-lww-b', kdfIterations: ITER }
		});
		await b.ready;
		timeB.advance(50);
		await requireOpen(b.state).apply({ kind: 'update', id, patch: { title: 'From B' } });
		await requireOpen(b.state).sync.now();

		timeA.advance(2_000);
		await requireOpen(a.state).apply({ kind: 'update', id, patch: { title: 'From A newer' } });
		await requireOpen(a.state).sync.now();
		await requireOpen(b.state).sync.now();

		expect(requireOpen(a.state).get(id)?.title).toBe('From A newer');
		expect(requireOpen(b.state).get(id)?.title).toBe('From A newer');
		a.close();
		b.close();
	});

	it('refuses a second salt insert for the same account', async () => {
		const mirror = newMirror();
		const cloud = cloudPortFor(mirror, ACCOUNT.id);
		await vaultKeyFromPassword({ password: PASSWORD, cloud, iterations: ITER });
		await expect(
			cloud.createVaultParams({ salt: 'AAAA', checkBlob: 'BBBB' })
		).rejects.toMatchObject({ error: { code: 'vault-exists' } });
		await expect(
			mirror.createVaultParams(ACCOUNT.id, { salt: 'AAAA', checkBlob: 'BBBB' })
		).rejects.toBeInstanceOf(VaultParamsExistError);
	});

	it('uploads only dirty records', async () => {
		const mirror = newMirror();
		const inner = cloudPortFor(mirror, ACCOUNT.id);
		const uploads: string[][] = [];
		const cloud = {
			...inner,
			async sync(batch: SyncPushBatch) {
				uploads.push(batch.changes.map((c) => c.recordId));
				return inner.sync(batch);
			}
		};
		const key = await vaultKeyFromPassword({ password: PASSWORD, cloud, iterations: ITER });
		const ws = openWorkspace({
			account: ACCOUNT,
			vaultKey: key,
			adapters: { cloud, clock: clock(), dbName: 'ws-dirty', kdfIterations: ITER }
		});
		await ws.ready;
		const id1 = await requireOpen(ws.state).apply({
			kind: 'create',
			title: 'First',
			important: true,
			urgent: true
		});
		await requireOpen(ws.state).sync.now();
		const id2 = await requireOpen(ws.state).apply({
			kind: 'create',
			title: 'Second',
			important: false,
			urgent: false
		});
		await requireOpen(ws.state).sync.now();
		const nonempty = uploads.filter((ids) => ids.length > 0);
		expect(nonempty[0]).toEqual([id1]);
		expect(nonempty[1]).toEqual([id2]);
		ws.close();
	});

	it('leaves ciphertext undecryptable after a password reset', async () => {
		const mirror = newMirror();
		const cloud = cloudPortFor(mirror, ACCOUNT.id);
		const key = await vaultKeyFromPassword({ password: PASSWORD, cloud, iterations: ITER });
		const ws = openWorkspace({
			account: ACCOUNT,
			vaultKey: key,
			adapters: { cloud, clock: clock(), dbName: 'ws-reset', kdfIterations: ITER }
		});
		await ws.ready;
		await requireOpen(ws.state).apply({
			kind: 'create',
			title: 'Secret',
			important: true,
			urgent: true
		});
		await requireOpen(ws.state).sync.now();
		ws.close();

		const stillThere = await cloud.sync({ lastVersion: 0, changes: [] });
		expect(stillThere.changes).toHaveLength(1);

		await expect(
			vaultKeyFromPassword({ password: 'brand-new-reset-password', cloud, iterations: ITER })
		).rejects.toMatchObject({ error: { code: 'wrong-passphrase' } });

		const original = await vaultKeyFromPassword({ password: PASSWORD, cloud, iterations: ITER });
		const again = openWorkspace({
			account: ACCOUNT,
			vaultKey: original,
			adapters: { cloud, clock: clock(9_000), dbName: 'ws-reset-old', kdfIterations: ITER }
		});
		await again.ready;
		expect(requireOpen(again.state).matrix.cells['do-now'].tasks.map((t) => t.title)).toEqual([
			'Secret'
		]);
		again.close();
	});

	it('signOut does not call navigation', async () => {
		const mirror = newMirror();
		const cloud = cloudPortFor(mirror, ACCOUNT.id);
		const key = await vaultKeyFromPassword({ password: PASSWORD, cloud, iterations: ITER });
		let signedOut = false;
		const ws = openWorkspace({
			account: ACCOUNT,
			vaultKey: key,
			adapters: {
				cloud,
				clock: clock(),
				dbName: 'ws-signout',
				kdfIterations: ITER,
				onSignOut: async () => {
					signedOut = true;
				}
			}
		});
		await ws.ready;
		await requireOpen(ws.state).signOut();
		expect(signedOut).toBe(true);
		expect(ws.state.status).toBe('booting');
		ws.close();
	});
});

describe('auth verifier', () => {
	it('differs from a vault key stretch so the server cannot reuse it', async () => {
		const email = 'a@b.co';
		const verifier = await deriveAuthVerifier(PASSWORD, email, ITER);
		const salt = new Uint8Array(16);
		const vaultBits = await deriveVaultKey(PASSWORD, salt, ITER);
		expect(verifier.length).toBeGreaterThan(8);
		expect(vaultBits).toBeInstanceOf(CryptoKey);
		const other = await deriveAuthVerifier(PASSWORD, 'other@b.co', ITER);
		expect(other).not.toBe(verifier);
	});
});
