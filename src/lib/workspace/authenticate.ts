import { createVaultSession } from './vault-session';
import { httpCloud } from './http-cloud';

export const vaultSession = createVaultSession({ cloud: httpCloud() });

export { createVaultSession } from './vault-session';
export type { VaultSession, VaultSessionDeps } from './vault-session';
