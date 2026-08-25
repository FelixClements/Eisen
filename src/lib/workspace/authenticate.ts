import { createVaultSession } from './vault-session';
import { httpCloud } from './http-cloud';

export const vaultSession = createVaultSession({ cloud: httpCloud() });

export { createVaultSession } from './vault-session';
export type { VaultSession, VaultSessionDeps } from './vault-session';

/** @deprecated Use vaultSession.unlock */
export async function authenticateWithVault(opts: {
	mode: 'sign-in' | 'sign-up';
	email: string;
	password: string;
	name?: string;
}): Promise<{ id: string }> {
	const { accountId } = await vaultSession.unlock(opts);
	return { id: accountId };
}
