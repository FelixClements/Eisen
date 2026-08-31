import { VaultParamsExistError, type VaultParams } from '$lib/sync/types';
import type { MirrorDatabasePort } from './types';

export function createVaultParamsApi(database: MirrorDatabasePort) {
	return {
		async getVaultParams(accountId: string) {
			return database.getVaultParams(accountId);
		},
		async createVaultParams(accountId: string, params: VaultParams) {
			const result = await database.insertVaultParams(accountId, params);
			if (result === 'exists') throw new VaultParamsExistError();
		}
	};
}
