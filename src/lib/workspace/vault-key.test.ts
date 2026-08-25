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
});
