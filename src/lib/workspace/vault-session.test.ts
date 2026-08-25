import 'fake-indexeddb/auto';
import { describe, expect, it } from 'vitest';
import { createEncryptedMirror } from '$lib/server/encrypted-mirror';
import {
	memoryMirrorDatabase,
	memoryRecoveryObjects,
	recordingPushDispatch
} from '$lib/server/adapters/memory';
import { cloudPortFor } from './memory-cloud';
import { createVaultSession } from './vault-session';
import { createWrappedKeyStore } from './wrapped-key-store';
import { vaultKeyFromPassword } from './vault-key';
import { requireOpen } from './workspace';

const ITER = 1;
const ACCOUNT = 'acct-1';
const PASSWORD = 'correct horse';

function newMirror() {
	return createEncryptedMirror({
		database: memoryMirrorDatabase(),
		recoveryObjects: memoryRecoveryObjects(),
		pushDispatch: recordingPushDispatch()
	});
}

describe('VaultSession', () => {
	it('resume returns a wrapped key', async () => {
		const mirror = newMirror();
		const cloud = cloudPortFor(mirror, ACCOUNT);
		const keyStore = createWrappedKeyStore({ dbName: 'vs-resume' });
		const key = await vaultKeyFromPassword({ password: PASSWORD, cloud, iterations: ITER });
		await keyStore.put(ACCOUNT, key);
		const session = createVaultSession({ cloud, keyStore, kdfIterations: ITER, dbName: 'vs-ws-resume' });
		expect(await session.resume(ACCOUNT)).toBeTruthy();
	});

	it('resume returns null when no wrapped key exists', async () => {
		const mirror = newMirror();
		const cloud = cloudPortFor(mirror, ACCOUNT);
		const session = createVaultSession({
			cloud,
			keyStore: createWrappedKeyStore({ dbName: 'vs-empty' }),
			kdfIterations: ITER
		});
		expect(await session.resume(ACCOUNT)).toBeNull();
	});

	it('lock clears the wrapped key', async () => {
		const mirror = newMirror();
		const cloud = cloudPortFor(mirror, ACCOUNT);
		const keyStore = createWrappedKeyStore({ dbName: 'vs-lock' });
		const key = await vaultKeyFromPassword({ password: PASSWORD, cloud, iterations: ITER });
		await keyStore.put(ACCOUNT, key);
		const session = createVaultSession({ cloud, keyStore, kdfIterations: ITER });
		await session.lock(ACCOUNT);
		expect(await session.resume(ACCOUNT)).toBeNull();
	});

	it('openWorkspace after resume loads tasks', async () => {
		const mirror = newMirror();
		const cloud = cloudPortFor(mirror, ACCOUNT);
		const keyStore = createWrappedKeyStore({ dbName: 'vs-open' });
		const key = await vaultKeyFromPassword({ password: PASSWORD, cloud, iterations: ITER });
		await keyStore.put(ACCOUNT, key);
		const session = createVaultSession({ cloud, keyStore, kdfIterations: ITER, dbName: 'vs-ws-open' });
		const resumed = await session.resume(ACCOUNT);
		expect(resumed).toBeTruthy();
		const ws = session.openWorkspace(ACCOUNT, resumed!, {});
		await ws.ready;
		const id = await requireOpen(ws.state).apply({
			kind: 'create',
			title: 'From session',
			important: true,
			urgent: true
		});
		expect(id).toBeTruthy();
		ws.close();
	});
});
