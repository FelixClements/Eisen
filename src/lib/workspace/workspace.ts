import { EisenErrorException, type TaskEdit, type TaskId, type TaskView } from './types';
import { buildMatrix, sortTasks } from './matrix';
import type { Clock, CloudPort, ReminderEnableResult, RemindersPort, WakePort } from './ports';
import { nullReminders, systemClock } from './ports';
import { createTaskCodec, type CatalogTask } from './task-codec';
import { createTaskRepository } from './task-repository';
import { createTaskCatalog } from './task-catalog';
import { createSyncEngine } from './sync-engine';
import { createRecoveryService } from './recovery-service';
import type { BackupRef, Outcome } from './types';
import type { Matrix } from './types';
import { syncReminderCache } from '$lib/reminder-cache';

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
		export(recoveryPassphrase: string): Promise<Outcome<Blob>>;
		backup(recoveryPassphrase: string): Promise<Outcome<BackupRef>>;
		list(): Promise<Outcome<BackupRef[]>>;
		restoreCloud(packageId: string, recoveryPassphrase: string): Promise<Outcome<void>>;
		import(file: File, recoveryPassphrase: string): Promise<Outcome<void>>;
	};
	reminders: {
		permission(): 'unsupported' | 'default' | 'granted' | 'denied';
		enable(): Promise<ReminderEnableResult>;
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
	wake?: WakePort;
	reminders?: RemindersPort;
	clock?: Clock;
	dbName?: string;
	indexedDB?: IDBFactory;
	IDBKeyRange?: typeof globalThis.IDBKeyRange;
	onSignOut?: () => Promise<void>;
	kdfIterations?: number;
};

export function openWorkspace(opts: {
	account: { id: string };
	vaultKey: CryptoKey;
	adapters: WorkspaceAdapters;
}): Workspace {
	const accountId = opts.account.id;
	const codec = createTaskCodec(opts.vaultKey);
	const repo = createTaskRepository({
		dbName: opts.adapters.dbName,
		indexedDB: opts.adapters.indexedDB,
		IDBKeyRange: opts.adapters.IDBKeyRange
	});
	const cloud = opts.adapters.cloud;
	const wake = opts.adapters.wake ?? cloud;
	const reminders = opts.adapters.reminders ?? nullReminders;
	const clock = opts.adapters.clock ?? systemClock;
	const kdfIterations = opts.adapters.kdfIterations ?? 600_000;
	const listeners = new Set<() => void>();

	let closed = false;
	let filter = '';
	let deviceId = '';
	let lastVersion = 0;
	let remindersPushEnabled = false;
	let status: WorkspaceState['status'] = 'booting';

	function notify() {
		for (const fn of listeners) fn();
	}

	async function persist(task: CatalogTask, syncVersion = 0) {
		task.blob = await codec.encryptTask(task);
		await repo.put(codec.toStoredRow(task, accountId, syncVersion));
	}

	let syncEngine!: ReturnType<typeof createSyncEngine>;

	function upcomingReminders(now = clock.now()) {
		return [...catalog.all()]
			.filter((t) => !t.deleted && !t.completed && !t.archived && t.remindAt)
			.map((t) => ({ id: t.id, title: t.title, remindAt: t.remindAt as number }));
	}

	async function refreshReminderCache() {
		if (!remindersPushEnabled) return;
		try {
			await syncReminderCache(upcomingReminders());
		} catch (err) {
			console.error('reminder cache sync failed:', err);
		}
	}

	const catalog = createTaskCatalog({
		codec,
		clock,
		getDeviceId: () => deviceId,
		isOpen: () => status === 'open',
		persist,
		onChanged: () => {
			notify();
			void refreshReminderCache();
			void scheduleWake();
			void syncEngine.run();
		}
	});

	async function scheduleWake() {
		const now = clock.now();
		const upcoming = [...catalog.all()]
			.filter((t) => !t.deleted && !t.completed && !t.archived && t.remindAt && t.remindAt > now)
			.sort((a, b) => (a.remindAt ?? 0) - (b.remindAt ?? 0));
		if (upcoming.length === 0) return;
		try {
			await wake.scheduleWake({
				deviceId,
				wakeAt: upcoming[0].remindAt as number,
				nonce: clock.uuid()
			});
		} catch (err) {
			console.error('scheduleWake failed:', err);
		}
	}

	syncEngine = createSyncEngine({
		cloud,
		codec,
		repo,
		accountId,
		getDeviceId: () => deviceId,
		getLastVersion: () => lastVersion,
		setLastVersion: (v) => {
			lastVersion = v;
		},
		getTasks: () => catalog.all(),
		findTask: (id) => catalog.find(id),
		ensurePersisted: persist,
		applyRemote: async (task, syncVersion) => {
			catalog.set(task);
			await repo.put(codec.toStoredRow(task, accountId, syncVersion));
			notify();
		},
		onNotify: notify,
		scheduleWake
	});

	const recovery = createRecoveryService({
		accountId,
		codec,
		cloud,
		clock,
		kdfIterations,
		getDeviceId: () => deviceId,
		getTasks: () => catalog.all(),
		persist,
		setTask: (task) => catalog.set(task),
		onImported: notify,
		triggerSync: () => void syncEngine.run()
	});

	function snapshotOpen(): OpenState {
		const all = catalog.views();
		const matrix = buildMatrix(all, filter);
		const completed = sortTasks(all.filter((t) => t.completed && !t.archived)).sort(
			(a, b) => b.updatedAt - a.updatedAt
		);
		const archived = [...all.filter((t) => t.archived)].sort((a, b) => b.updatedAt - a.updatedAt);
		return {
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
			get: (id) => catalog.get(id),
			apply: (edit) => catalog.apply(edit),
			sync: {
				get phase() {
					return syncEngine.phase;
				},
				get lastError() {
					return syncEngine.lastError;
				},
				now: () => syncEngine.run()
			},
			recovery: {
				export: (recoveryPassphrase) => recovery.exportRecoveryPackage(recoveryPassphrase),
				backup: (recoveryPassphrase) => recovery.backupToCloud(recoveryPassphrase),
				list: () => recovery.listCloudBackups(),
				restoreCloud: (packageId, recoveryPassphrase) =>
					recovery.restoreFromCloud(packageId, recoveryPassphrase),
				import: (file, recoveryPassphrase) => recovery.importRecoveryPackage(file, recoveryPassphrase)
			},
			reminders: {
				permission: () => reminders.permission(),
				enable: enableReminders
			},
			dueReminders() {
				const now = clock.now();
				return all
					.filter((t) => !t.completed && !t.archived && t.remindAt && t.remindAt <= now)
					.map((t) => ({ id: t.id, title: t.title }));
			},
			signOut
		};
	}

	function getState(): WorkspaceState {
		if (status === 'booting') return { status: 'booting' };
		return snapshotOpen();
	}

	async function loadLocal() {
		const meta = await repo.ensureMeta(accountId, () => clock.uuid());
		deviceId = meta.deviceId;
		lastVersion = meta.lastVersion;
		const rows = await repo.loadRows(accountId);
		for (const row of rows) {
			const payload = await codec.decryptPayload(row.blob);
			if (!payload) continue;
			const task = codec.fromPayload(payload, {
				id: row.id,
				updatedAt: row.updatedAt,
				deleted: row.deleted === 1,
				dirty: row.dirty === 1
			});
			task.blob = row.blob;
			catalog.set(task);
		}
	}

	async function enableReminders(): Promise<ReminderEnableResult> {
		const initial = reminders.permission();
		if (initial === 'unsupported') return 'unsupported';
		const perm = await reminders.request();
		if (perm === 'unsupported') return 'unsupported';
		if (perm === 'denied') return 'denied';
		const sub = await reminders.subscribe();
		if (!sub) return 'vapid-missing';
		try {
			await wake.registerPush({ deviceId, ...sub });
		} catch (err) {
			console.error('registerPush failed:', err);
			return 'server-error';
		}
		remindersPushEnabled = true;
		await refreshReminderCache();
		await scheduleWake();
		return 'granted';
	}

	async function signOut(): Promise<void> {
		catalog.clear();
		status = 'booting';
		notify();
		if (opts.adapters.onSignOut) await opts.adapters.onSignOut();
	}

	const ready = (async () => {
		await loadLocal();
		status = 'open';
		notify();
		if (reminders.permission() === 'granted') {
			remindersPushEnabled = true;
			await refreshReminderCache();
		}
		await syncEngine.run();
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
			repo.close();
		}
	};
}

export function requireOpen(state: WorkspaceState): OpenState {
	if (state.status !== 'open') throw new Error('Workspace is not open');
	return state;
}
