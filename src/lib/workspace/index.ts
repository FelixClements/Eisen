export { openWorkspace, requireOpen, type Workspace, type WorkspaceState, type OpenState } from './workspace';
export { vaultKeyFromPassword } from './vault-key';
export { vaultSession, createVaultSession } from './authenticate';
export type { VaultSession, VaultSessionDeps } from './vault-session';
export { cloudPortFor } from './memory-cloud';
export { flagsFromQuadrant, QUADRANT_META, QUADRANT_ORDER, quadrantOf } from './matrix';
export type {
	CloudPort,
	VaultParamsPort,
	SyncPort,
	BackupPort,
	WakePort,
	RemindersPort,
	Clock,
	VaultParams
} from './ports';
export { nullReminders, systemClock } from './ports';
export type { TaskEdit, TaskView, Quadrant, Matrix, EisenError, BackupRef, Outcome } from './types';
export { EisenErrorException } from './types';
