export type Quadrant = 'do-now' | 'schedule' | 'delegate' | 'eliminate';

export type TaskId = string;

export type TaskView = Readonly<{
	id: TaskId;
	title: string;
	notes: string;
	tag: string;
	important: boolean;
	urgent: boolean;
	quadrant: Quadrant;
	dueAt: number | null;
	remindAt: number | null;
	pinned: boolean;
	completed: boolean;
	archived: boolean;
	createdAt: number;
	updatedAt: number;
	syncState: 'local' | 'synced';
}>;

export type MatrixCell = {
	label: string;
	hint: string;
	shortcut: string;
	tasks: TaskView[];
};

export type Matrix = {
	order: readonly Quadrant[];
	cells: Record<Quadrant, MatrixCell>;
	total: number;
};

export type TaskEdit =
	| {
			kind: 'create';
			title: string;
			notes?: string;
			important: boolean;
			urgent: boolean;
			dueAt?: number | null;
			remindAt?: number | null;
			tag?: string;
			pinned?: boolean;
	  }
	| {
			kind: 'update';
			id: TaskId;
			patch: Partial<
				Pick<
					TaskView,
					'title' | 'notes' | 'tag' | 'important' | 'urgent' | 'dueAt' | 'remindAt' | 'pinned'
				>
			>;
	  }
	| { kind: 'complete'; id: TaskId; done: boolean }
	| { kind: 'archive'; id: TaskId; archived: boolean }
	| { kind: 'delete'; id: TaskId };

export type { BackupRef, SyncRecord } from '$lib/sync/types';
export { VaultParamsExistError } from '$lib/sync/types';

export type EisenError =
	| { code: 'wrong-passphrase' }
	| { code: 'weak-passphrase'; reason: string }
	| { code: 'vault-exists' }
	| { code: 'package-corrupt' }
	| { code: 'package-wrong-account' }
	| { code: 'session-expired' }
	| { code: 'offline' }
	| { code: 'server'; status: number }
	| { code: 'storage'; message: string }
	| { code: 'invalid-task'; reason: string }
	| { code: 'task-missing' };

export class EisenErrorException extends Error {
	readonly error: EisenError;
	constructor(error: EisenError) {
		super(error.code);
		this.error = error;
	}
}

export type Outcome<T> = { ok: true; value: T } | { ok: false; error: EisenError };

export type EncryptedTaskPayload = {
	title: string;
	notes: string;
	tag: string;
	important: boolean;
	urgent: boolean;
	dueAt: number | null;
	remindAt: number | null;
	pinned: boolean;
	completed: boolean;
	archived: boolean;
	createdAt: number;
	deviceId: string;
};
