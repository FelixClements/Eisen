import { describe, expect, it, vi } from 'vitest';
import { vaultKeyFromPassword } from './vault-key';
import type { VaultParamsPort } from './ports';

describe('vaultKeyFromPassword', () => {
	it('uses only VaultParamsPort methods', async () => {
		const getVaultParams = vi.fn(async () => null);
		const createVaultParams = vi.fn(async () => {});
		const cloud: VaultParamsPort = { getVaultParams, createVaultParams };
		await vaultKeyFromPassword({ password: 'long-enough-password', cloud, iterations: 1 });
		expect(getVaultParams).toHaveBeenCalled();
		expect(createVaultParams).toHaveBeenCalled();
	});

	it('retries derive after a first-device vault-exists race', async () => {
		const { toBase64 } = await import('$lib/crypto');
		const { deriveVaultKey, makeCheckBlob, newKdfSalt } = await import('$lib/crypto');
		const { EisenErrorException } = await import('./types');
		const salt = await newKdfSalt();
		const winnerKey = await deriveVaultKey('long-enough-password', salt, 1);
		const checkBlob = await makeCheckBlob(winnerKey);
		const params = { salt: toBase64(salt), checkBlob };
		let gets = 0;
		const cloud: VaultParamsPort = {
			getVaultParams: async () => {
				gets += 1;
				return gets === 1 ? null : params;
			},
			createVaultParams: async () => {
				throw new EisenErrorException({ code: 'vault-exists' });
			}
		};
		const key = await vaultKeyFromPassword({
			password: 'long-enough-password',
			cloud,
			iterations: 1
		});
		expect(key).toBeTruthy();
		expect(gets).toBe(2);
	});
});
