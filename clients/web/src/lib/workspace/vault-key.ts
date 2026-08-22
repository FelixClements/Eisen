import {
	deriveVaultKey,
	fromBase64,
	makeCheckBlob,
	newKdfSalt,
	toBase64,
	verifyCheckBlob,
	VAULT_KDF_ITERATIONS
} from '$lib/crypto';
import type { CloudPort } from './ports';
import { EisenErrorException } from './types';
import { EISEN_DB_NAME, EisenWebDB } from './db';

export async function vaultKeyFromPassword(opts: {
	password: string;
	cloud: CloudPort;
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

export async function wrapVaultKey(opts: {
	accountId: string;
	key: CryptoKey;
	dbName?: string;
	indexedDB?: IDBFactory;
	IDBKeyRange?: typeof globalThis.IDBKeyRange;
}): Promise<void> {
	const db = new EisenWebDB(opts.dbName ?? EISEN_DB_NAME, opts.indexedDB, opts.IDBKeyRange);
	await db.wrappedKeys.put({ accountId: opts.accountId, key: opts.key });
	db.close();
}

export async function unwrapVaultKey(opts: {
	accountId: string;
	dbName?: string;
	indexedDB?: IDBFactory;
	IDBKeyRange?: typeof globalThis.IDBKeyRange;
}): Promise<CryptoKey | null> {
	const db = new EisenWebDB(opts.dbName ?? EISEN_DB_NAME, opts.indexedDB, opts.IDBKeyRange);
	const row = await db.wrappedKeys.get(opts.accountId);
	db.close();
	return row?.key ?? null;
}

export async function clearWrappedKey(opts: {
	accountId: string;
	dbName?: string;
	indexedDB?: IDBFactory;
	IDBKeyRange?: typeof globalThis.IDBKeyRange;
}): Promise<void> {
	const db = new EisenWebDB(opts.dbName ?? EISEN_DB_NAME, opts.indexedDB, opts.IDBKeyRange);
	await db.wrappedKeys.delete(opts.accountId);
	db.close();
}
