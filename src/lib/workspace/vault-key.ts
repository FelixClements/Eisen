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
		try {
			await opts.cloud.createVaultParams({ salt: toBase64(salt), checkBlob });
			return key;
		} catch (err) {
			if (!(err instanceof EisenErrorException) || err.error.code !== 'vault-exists') throw err;
			const winner = await opts.cloud.getVaultParams();
			if (!winner) throw err;
			const retryKey = await deriveVaultKey(opts.password, fromBase64(winner.salt), iterations);
			const ok = await verifyCheckBlob(winner.checkBlob, retryKey);
			if (!ok) throw new EisenErrorException({ code: 'wrong-passphrase' });
			return retryKey;
		}
	}
	const key = await deriveVaultKey(opts.password, fromBase64(existing.salt), iterations);
	const ok = await verifyCheckBlob(existing.checkBlob, key);
	if (!ok) throw new EisenErrorException({ code: 'wrong-passphrase' });
	return key;
}

