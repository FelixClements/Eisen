import { decrypt, encrypt } from '$lib/crypto';
import { EisenErrorException, type EncryptedTaskPayload, type TaskEdit, type TaskId, type TaskView } from './types';
import { buildMatrix, quadrantOf, sortTasks, utf8Bytes } from './matrix';
import { remoteWins } from './merge';
import type { Clock, CloudPort, RemindersPort } from './ports';
import { nullReminders, systemClock } from './ports';
import { EISEN_DB_NAME, EisenWebDB, type StoredTask } from './db';
import type { BackupRef, Outcome } from './types';
import type { Matrix } from './types';

const TITLE_MAX = 256;
const NOTES_MAX = 4096;

export type OpenState = {
	status: 'open';
	filter: string;
	readonly matrix: Matrix;
	readonly history: { completed: TaskView[]; archived: TaskView[] };
	get(id: TaskId): TaskView | undefined;
	apply(edit: TaskEdit): Promise<TaskId>;
	sync: {
		now(): Promise<void>;
		phase: 'idle' | 'syncing' | 'offline' | 'error';
		lastError: import('./types').EisenError | null;
	};
	recovery: {
		export(passphrase: string): Promise<Outcome<Blob>>;
		backup(passphrase: string): Promise<Outcome<BackupRef>>;
		list(): Promise<Outcome<BackupRef[]>>;
		restoreCloud(packageId: string, passphrase: string): Promise<Outcome<void>>;
		import(file: File, passphrase: string): Promise<Outcome<void>>;
	};
	reminders: {
		enable(): Promise<void>;
	};
	dueReminders(): { id: string; title: string }[];
	signOut(): Promise<void>;
};

export type WorkspaceState = { status: 'booting' } | OpenState;

export type Workspace = {
	readonly ready: Promise<void>;
	get state(): WorkspaceState;
	subscribe(fn: () => void): () => void;
	close(): void;
};

export type WorkspaceAdapters = {
	cloud: CloudPort;
	reminders?: RemindersPort;
	clock?: Clock;
	dbName?: string;
	indexedDB?: IDBFactory;
	IDBKeyRange?: typeof globalThis.IDBKeyRange;
	onSignOut?: () => Promise<void>;
	kdfIterations?: number;
};

type MemoryTask = {
	id: string;
	title: string;
	notes: string;
	tag: string;
	important: boolean;
	urgent: boolean;
	quadrant: import('./types').Quadrant;
	dueAt: number | null;
	remindAt: number | null;
	pinned: boolean;
	completed: boolean;
	archived: boolean;
	createdAt: number;
	updatedAt: number;
	syncState: 'local' | 'synced';
	deviceId: string;
	deleted: boolean;
	dirty: boolean;
	blob: string;
};

function utf8TooLong(value: string, max: number): boolean {
	return utf8Bytes(value) > max;
}

export function openWorkspace(opts: {
	account: { id: string };
	vaultKey: CryptoKey;
	adapters: WorkspaceAdapters;
}): Workspace {
	const accountId = opts.account.id;
	const key = opts.vaultKey;
	const cloud = opts.adapters.cloud;
	const reminders = opts.adapters.reminders ?? nullReminders;
	const clock = opts.adapters.clock ?? systemClock;
	const kdfIterations = opts.adapters.kdfIterations ?? 600_000;
	const listeners = new Set<() => void>();
	const db = new EisenWebDB(
		opts.adapters.dbName ?? EISEN_DB_NAME,
		opts.adapters.indexedDB,
		opts.adapters.IDBKeyRange
	);

	let closed = false;
	let filter = '';
	let phase: OpenState['sync']['phase'] = 'idle';
	let lastError: OpenState['sync']['lastError'] = null;
	let tasks = new Map<string, MemoryTask>();
	let deviceId = '';
	let lastVersion = 0;
	let syncing: Promise<void> | null = null;
	let status: WorkspaceState['status'] = 'booting';

	function notify() {
		for (const fn of listeners) fn();
	}

	function views(): TaskView[] {
		return [...tasks.values()]
			.filter((t) => !t.deleted)
			.map((t) => ({
				id: t.id,
				title: t.title,
				notes: t.notes,
				tag: t.tag,
				important: t.important,
				urgent: t.urgent,
				quadrant: t.quadrant,
				dueAt: t.dueAt,
				remindAt: t.remindAt,
				pinned: t.pinned,
				completed: t.completed,
				archived: t.archived,
				createdAt: t.createdAt,
				updatedAt: t.updatedAt,
				syncState: t.dirty ? 'local' : 'synced'
			}));
	}

	function snapshotOpen(): OpenState {
		const all = views();
		const matrix = buildMatrix(all, filter);
		const completed = sortTasks(all.filter((t) => t.completed && !t.archived)).sort(
			(a, b) => b.updatedAt - a.updatedAt
		);
		const archived = [...all.filter((t) => t.archived)].sort((a, b) => b.updatedAt - a.updatedAt);
		const sync = {
			phase,
			lastError,
			now: () => runSync()
		};
		const open: OpenState = {
			status: 'open',
			get filter() {
				return filter;
			},
			set filter(value: string) {
				filter = value;
				notify();
			},
			get matrix() {
				return matrix;
			},
			history: { completed, archived },
			get(id) {
				return all.find((t) => t.id === id);
			},
			apply,
			sync,
			recovery: {
				export: exportPackage,
				backup: backupToCloud,
				list: listCloud,
				restoreCloud: restoreFromCloud,
				import: importPackage
			},
			reminders: { enable: enableReminders },
			dueReminders() {
				const now = clock.now();
				return views()
					.filter((t) => !t.completed && !t.archived && t.remindAt && t.remindAt <= now)
					.map((t) => ({ id: t.id, title: t.title }));
			},
			signOut
		};
		return open;
	}

	function getState(): WorkspaceState {
		if (status === 'booting') return { status: 'booting' };
		return snapshotOpen();
	}

	async function persist(task: MemoryTask) {
		const payload: EncryptedTaskPayload = {
			title: task.title,
			notes: task.notes,
			tag: task.tag,
			important: task.important,
			urgent: task.urgent,
			dueAt: task.dueAt,
			remindAt: task.remindAt,
			pinned: task.pinned,
			completed: task.completed,
			archived: task.archived,
			createdAt: task.createdAt,
			deviceId: task.deviceId
		};
		const blob = await encrypt(JSON.stringify(payload), key);
		task.blob = blob;
		const row: StoredTask = {
			id: task.id,
			accountId,
			updatedAt: task.updatedAt,
			deleted: task.deleted ? 1 : 0,
			blob,
			dirty: task.dirty ? 1 : 0,
			syncVersion: 0
		};
		await db.tasks.put(row);
	}

	async function loadLocal() {
		let meta = await db.meta.get(accountId);
		if (!meta) {
			meta = { accountId, deviceId: clock.uuid(), lastVersion: 0 };
			await db.meta.put(meta);
		}
		deviceId = meta.deviceId;
		lastVersion = meta.lastVersion;
		const rows = await db.tasks.where('accountId').equals(accountId).toArray();
		for (const row of rows) {
			try {
				const payload = JSON.parse(await decrypt(row.blob, key)) as EncryptedTaskPayload;
				tasks.set(row.id, {
					id: row.id,
					title: payload.title,
					notes: payload.notes,
					tag: payload.tag,
					important: payload.important,
					urgent: payload.urgent,
					quadrant: quadrantOf(payload),
					dueAt: payload.dueAt,
					remindAt: payload.remindAt,
					pinned: payload.pinned,
					completed: payload.completed,
					archived: payload.archived,
					createdAt: payload.createdAt,
					updatedAt: row.updatedAt,
					syncState: row.dirty ? 'local' : 'synced',
					deviceId: payload.deviceId,
					deleted: row.deleted === 1,
					dirty: row.dirty === 1,
					blob: row.blob
				});
			} catch {
				// skip undecryptable rows
			}
		}
	}

	function validateTitle(title: string) {
		const trimmed = title.trim();
		if (!trimmed) throw new EisenErrorException({ code: 'invalid-task', reason: 'Title is required' });
		if (utf8TooLong(trimmed, TITLE_MAX)) {
			throw new EisenErrorException({ code: 'invalid-task', reason: 'Title is too long' });
		}
		return trimmed;
	}

	function validateNotes(notes: string) {
		if (utf8TooLong(notes, NOTES_MAX)) {
			throw new EisenErrorException({ code: 'invalid-task', reason: 'Notes are too long' });
		}
		return notes;
	}

	function requireTask(id: string): MemoryTask {
		const task = tasks.get(id);
		if (!task || task.deleted) throw new EisenErrorException({ code: 'task-missing' });
		return task;
	}

	async function apply(edit: TaskEdit): Promise<TaskId> {
		if (status !== 'open') throw new EisenErrorException({ code: 'storage', message: 'Workspace is not open' });
		const now = clock.now();
		if (edit.kind === 'create') {
			if (edit.remindAt && edit.remindAt < now) {
				throw new EisenErrorException({ code: 'invalid-task', reason: 'Reminder is in the past' });
			}
			const id = clock.uuid();
			const title = validateTitle(edit.title);
			const notes = validateNotes(edit.notes ?? '');
			const task: MemoryTask = {
				id,
				title,
				notes,
				tag: (edit.tag ?? '').trim(),
				important: edit.important,
				urgent: edit.urgent,
				quadrant: quadrantOf({ important: edit.important, urgent: edit.urgent }),
				dueAt: edit.dueAt ?? null,
				remindAt: edit.remindAt ?? null,
				pinned: edit.pinned ?? false,
				completed: false,
				archived: false,
				createdAt: now,
				updatedAt: now,
				syncState: 'local',
				deviceId,
				deleted: false,
				dirty: true,
				blob: ''
			};
			tasks.set(id, task);
			await persist(task);
			notify();
			void scheduleWake();
			void runSync();
			return id;
		}

		const task = requireTask(edit.id);
		if (edit.kind === 'update') {
			if (edit.patch.title !== undefined) task.title = validateTitle(edit.patch.title);
			if (edit.patch.notes !== undefined) task.notes = validateNotes(edit.patch.notes);
			if (edit.patch.tag !== undefined) task.tag = edit.patch.tag.trim();
			if (edit.patch.important !== undefined) task.important = edit.patch.important;
			if (edit.patch.urgent !== undefined) task.urgent = edit.patch.urgent;
			task.quadrant = quadrantOf(task);
			if (edit.patch.dueAt !== undefined) task.dueAt = edit.patch.dueAt;
			if (edit.patch.remindAt !== undefined) {
				if (edit.patch.remindAt && edit.patch.remindAt < now) {
					throw new EisenErrorException({ code: 'invalid-task', reason: 'Reminder is in the past' });
				}
				task.remindAt = edit.patch.remindAt;
			}
			if (edit.patch.pinned !== undefined) task.pinned = edit.patch.pinned;
		} else if (edit.kind === 'complete') {
			task.completed = edit.done;
			if (edit.done) task.archived = false;
		} else if (edit.kind === 'archive') {
			task.archived = edit.archived;
			if (edit.archived) task.completed = false;
		} else {
			task.deleted = true;
		}
		task.updatedAt = now;
		task.deviceId = deviceId;
		task.dirty = true;
		await persist(task);
		notify();
		void scheduleWake();
		void runSync();
		return task.id;
	}

	async function scheduleWake() {
		const now = clock.now();
		const upcoming = [...tasks.values()]
			.filter((t) => !t.deleted && !t.completed && !t.archived && t.remindAt && t.remindAt > now)
			.sort((a, b) => (a.remindAt ?? 0) - (b.remindAt ?? 0));
		if (upcoming.length === 0) return;
		try {
			await cloud.scheduleWake({
				deviceId,
				wakeAt: upcoming[0].remindAt as number,
				nonce: clock.uuid()
			});
		} catch {
			// wake is best-effort
		}
	}

	async function runSync(): Promise<void> {
		if (syncing) return syncing;
		syncing = (async () => {
			phase = 'syncing';
			lastError = null;
			notify();
			try {
				const changes = await Promise.all(
					[...tasks.values()]
						.filter((t) => t.dirty)
						.map(async (t) => {
							if (!t.blob) await persist(t);
							return {
								recordId: t.id,
								encryptedBlob: t.blob,
								modifiedAt: t.updatedAt,
								deleted: t.deleted ? 1 : 0
							};
						})
				);
				const result = await cloud.sync({ lastVersion, changes });
				for (const record of result.changes) {
					const syncVersion = record.syncVersion ?? 0;
					if (syncVersion === 0) continue;
					let payload: EncryptedTaskPayload;
					try {
						payload = JSON.parse(await decrypt(record.encryptedBlob, key)) as EncryptedTaskPayload;
					} catch {
						continue;
					}
					const existing = tasks.get(record.recordId);
					const incoming = { updatedAt: record.modifiedAt, deviceId: payload.deviceId };
					if (existing && !remoteWins(existing, incoming) && existing.dirty) {
						continue;
					}
					const next: MemoryTask = {
						id: record.recordId,
						title: payload.title,
						notes: payload.notes,
						tag: payload.tag,
						important: payload.important,
						urgent: payload.urgent,
						quadrant: quadrantOf(payload),
						dueAt: payload.dueAt,
						remindAt: payload.remindAt,
						pinned: payload.pinned,
						completed: payload.completed,
						archived: payload.archived,
						createdAt: payload.createdAt,
						updatedAt: record.modifiedAt,
						syncState: 'synced',
						deviceId: payload.deviceId,
						deleted: record.deleted === 1,
						dirty: false,
						blob: record.encryptedBlob
					};
					tasks.set(record.recordId, next);
					await db.tasks.put({
						id: next.id,
						accountId,
						updatedAt: next.updatedAt,
						deleted: next.deleted ? 1 : 0,
						blob: next.blob,
						dirty: 0,
						syncVersion
					});
				}
				for (const t of tasks.values()) {
					if (t.dirty && changes.some((c) => c.recordId === t.id && c.modifiedAt === t.updatedAt)) {
						t.dirty = false;
						t.syncState = 'synced';
						await db.tasks.update(t.id, { dirty: 0 });
					}
				}
				lastVersion = result.lastVersion;
				await db.meta.put({ accountId, deviceId, lastVersion });
				phase = 'idle';
				await scheduleWake();
			} catch (e) {
				if (e instanceof EisenErrorException) {
					lastError = e.error;
					phase = e.error.code === 'offline' ? 'offline' : 'error';
				} else {
					lastError = { code: 'server', status: 0 };
					phase = 'error';
				}
			} finally {
				syncing = null;
				notify();
			}
			if (phase === 'idle' && [...tasks.values()].some((t) => t.dirty)) {
				await runSync();
			}
		})();
		return syncing;
	}

	async function packageTasks(passphrase: string): Promise<Outcome<string>> {
		if (passphrase.length < 8) {
			return { ok: false, error: { code: 'weak-passphrase', reason: 'Passphrase must be at least 8 characters.' } };
		}
		try {
			const { deriveVaultKey, newKdfSalt, toBase64 } = await import('$lib/crypto');
			const salt = await newKdfSalt();
			const wrapKey = await deriveVaultKey(passphrase, salt, kdfIterations);
			const body = JSON.stringify({
				accountId,
				tasks: [...tasks.values()].map((t) => ({
					id: t.id,
					updatedAt: t.updatedAt,
					deleted: t.deleted,
					payload: {
						title: t.title,
						notes: t.notes,
						tag: t.tag,
						important: t.important,
						urgent: t.urgent,
						dueAt: t.dueAt,
						remindAt: t.remindAt,
						pinned: t.pinned,
						completed: t.completed,
						archived: t.archived,
						createdAt: t.createdAt,
						deviceId: t.deviceId
					}
				}))
			});
			const ciphertext = await encrypt(body, wrapKey);
			return {
				ok: true,
				value: JSON.stringify({
					version: 1,
					accountId,
					kdfSalt: toBase64(salt),
					ciphertext
				})
			};
		} catch (e) {
			return { ok: false, error: { code: 'storage', message: e instanceof Error ? e.message : 'export failed' } };
		}
	}

	async function exportPackage(passphrase: string): Promise<Outcome<Blob>> {
		const packed = await packageTasks(passphrase);
		if (!packed.ok) return packed;
		return { ok: true, value: new Blob([packed.value], { type: 'application/eisen-recovery' }) };
	}

	async function applyPackageText(text: string, passphrase: string): Promise<Outcome<void>> {
		try {
			const { deriveVaultKey, fromBase64, decrypt: dec } = await import('$lib/crypto');
			const pkg = JSON.parse(text) as {
				version: number;
				accountId: string;
				kdfSalt: string;
				ciphertext: string;
			};
			if (pkg.version !== 1) return { ok: false, error: { code: 'package-corrupt' } };
			if (pkg.accountId !== accountId) return { ok: false, error: { code: 'package-wrong-account' } };
			const wrapKey = await deriveVaultKey(passphrase, fromBase64(pkg.kdfSalt), kdfIterations);
			let plain: string;
			try {
				plain = await dec(pkg.ciphertext, wrapKey);
			} catch {
				return { ok: false, error: { code: 'wrong-passphrase' } };
			}
			const body = JSON.parse(plain) as {
				tasks: Array<{
					id: string;
					updatedAt: number;
					deleted: boolean;
					payload: EncryptedTaskPayload;
				}>;
			};
			for (const row of body.tasks) {
				const task: MemoryTask = {
					id: row.id,
					title: row.payload.title,
					notes: row.payload.notes,
					tag: row.payload.tag,
					important: row.payload.important,
					urgent: row.payload.urgent,
					quadrant: quadrantOf(row.payload),
					dueAt: row.payload.dueAt,
					remindAt: row.payload.remindAt,
					pinned: row.payload.pinned,
					completed: row.payload.completed,
					archived: row.payload.archived,
					createdAt: row.payload.createdAt,
					updatedAt: row.updatedAt,
					syncState: 'local',
					deviceId: row.payload.deviceId,
					deleted: row.deleted,
					dirty: true,
					blob: ''
				};
				tasks.set(task.id, task);
				await persist(task);
			}
			notify();
			void runSync();
			return { ok: true, value: undefined };
		} catch {
			return { ok: false, error: { code: 'package-corrupt' } };
		}
	}

	async function importPackage(file: File, passphrase: string): Promise<Outcome<void>> {
		return applyPackageText(await file.text(), passphrase);
	}

	async function backupToCloud(passphrase: string): Promise<Outcome<BackupRef>> {
		const packed = await packageTasks(passphrase);
		if (!packed.ok) return packed;
		try {
			const id = clock.uuid();
			const ref = await cloud.putBackup({ id, text: packed.value, deviceId });
			return { ok: true, value: ref };
		} catch {
			return { ok: false, error: { code: 'server', status: 0 } };
		}
	}

	async function listCloud(): Promise<Outcome<BackupRef[]>> {
		try {
			return { ok: true, value: await cloud.listBackups() };
		} catch {
			return { ok: false, error: { code: 'server', status: 0 } };
		}
	}

	async function restoreFromCloud(packageId: string, passphrase: string): Promise<Outcome<void>> {
		try {
			const text = await cloud.getBackup(packageId);
			return applyPackageText(text, passphrase);
		} catch {
			return { ok: false, error: { code: 'server', status: 0 } };
		}
	}

	async function enableReminders(): Promise<void> {
		const perm = await reminders.request();
		if (perm !== 'granted') return;
		const sub = await reminders.subscribe();
		if (sub) await cloud.registerPush(sub);
		await scheduleWake();
	}

	async function signOut(): Promise<void> {
		tasks.clear();
		status = 'booting';
		notify();
		if (opts.adapters.onSignOut) await opts.adapters.onSignOut();
	}

	const ready = (async () => {
		await loadLocal();
		status = 'open';
		notify();
		await runSync();
	})();

	return {
		ready,
		get state() {
			return getState();
		},
		subscribe(fn) {
			listeners.add(fn);
			return () => listeners.delete(fn);
		},
		close() {
			if (closed) return;
			closed = true;
			listeners.clear();
			db.close();
		}
	};
}

export function requireOpen(state: WorkspaceState): OpenState {
	if (state.status !== 'open') throw new Error('Workspace is not open');
	return state;
}
