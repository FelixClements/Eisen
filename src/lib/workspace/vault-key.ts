import {
	deriveVaultKey,
	fromBase64,
	makeCheckBlob,
	newKdfSalt,
	toBase64,
	verifyCheckBlob,
	VAULT_KDF_ITERATIONS
} from '$lib/crypto';
import type { VaultParamsPort } from './ports';
import { EisenErrorException } from './types';
import { createWrappedKeyStore } from './wrapped-key-store';

export async function vaultKeyFromPassword(opts: {
	password: string;
	cloud: VaultParamsPort;
	iterations?: number;
}): Promise<CryptoKey> {
	const iterations = opts.iterations ?? VAULT_KDF_ITERATIONS;
	const existing = await opts.cloud.getVaultParams();
	if (!existing) {
		const salt = await newKdfSalt();
		const key = await deriveVaultKey(opts.password, salt, iterations);
		const checkBlob = await makeCheckBlob(key);
		await opts.cloud.createVaultParams({ salt: toBase64(salt), checkBlob });
		return key;
	}
	const key = await deriveVaultKey(opts.password, fromBase64(existing.salt), iterations);
	const ok = await verifyCheckBlob(existing.checkBlob, key);
	if (!ok) throw new EisenErrorException({ code: 'wrong-passphrase' });
	return key;
}

const defaultStore = createWrappedKeyStore();

export async function wrapVaultKey(opts: {
	accountId: string;
	key: CryptoKey;
	dbName?: string;
	indexedDB?: IDBFactory;
	IDBKeyRange?: typeof globalThis.IDBKeyRange;
}): Promise<void> {
	const store = opts.dbName || opts.indexedDB || opts.IDBKeyRange
		? createWrappedKeyStore(opts)
		: defaultStore;
	await store.put(opts.accountId, opts.key);
}

export async function unwrapVaultKey(opts: {
	accountId: string;
	dbName?: string;
	indexedDB?: IDBFactory;
	IDBKeyRange?: typeof globalThis.IDBKeyRange;
}): Promise<CryptoKey | null> {
	const store = opts.dbName || opts.indexedDB || opts.IDBKeyRange
		? createWrappedKeyStore(opts)
		: defaultStore;
	return store.get(opts.accountId);
}

export async function clearWrappedKey(opts: {
	accountId: string;
	dbName?: string;
	indexedDB?: IDBFactory;
	IDBKeyRange?: typeof globalThis.IDBKeyRange;
}): Promise<void> {
	const store = opts.dbName || opts.indexedDB || opts.IDBKeyRange
		? createWrappedKeyStore(opts)
		: defaultStore;
	await store.delete(opts.accountId);
}
