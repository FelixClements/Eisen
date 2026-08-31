export {
	MirrorNotFoundError,
	PushSubscriptionConflictError,
	PushSubscriptionNotFoundError,
	type BackupMetaRow,
	type DueWake,
	type EncryptedMirror,
	type EncryptedMirrorPorts,
	type MirrorDatabasePort,
	type PushDispatchPort,
	type PushSubscriptionRow,
	type RecoveryObjectPort,
	type VaultRecordRow,
	type WakeScheduleRow
} from './mirror/types';
export { VaultParamsExistError } from '$lib/sync/types';

import type { EncryptedMirror, EncryptedMirrorPorts } from './mirror/types';
import { createVaultParamsApi } from './mirror/vault-params';
import { createRecordSyncApi } from './mirror/record-sync';
import { createRecoveryStoreApi } from './mirror/recovery';
import { createWakeClockApi } from './mirror/wake-clock';

export function createEncryptedMirror(ports: EncryptedMirrorPorts): EncryptedMirror {
	return {
		...createVaultParamsApi(ports.database),
		...createRecordSyncApi(ports.database),
		...createRecoveryStoreApi(ports.database, ports.recoveryObjects),
		...createWakeClockApi(ports.database, ports.pushDispatch)
	};
}
