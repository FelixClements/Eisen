import Dexie, { liveQuery, type Table } from 'dexie';

export type EisenhowerCategory = 'do_now' | 'schedule' | 'delegate' | 'eliminate';

export function eisenhowerCategory(task: { isImportant: boolean; isUrgent: boolean }): EisenhowerCategory {
	if (task.isImportant && task.isUrgent) return 'do_now';
	if (task.isImportant && !task.isUrgent) return 'schedule';
	if (!task.isImportant && task.isUrgent) return 'delegate';
	return 'eliminate';
}

export const categoryOrder: EisenhowerCategory[] = ['do_now', 'schedule', 'delegate', 'eliminate'];

export const categoryLabels: Record<
	EisenhowerCategory,
	{ title: string; desc: string; cls: string; shortcut: string }
> = {
	do_now: { title: 'Do Now', desc: 'Important & urgent', cls: 'do-now', shortcut: 'Q' },
	schedule: { title: 'Schedule', desc: 'Important, not urgent', cls: 'schedule', shortcut: 'W' },
	delegate: { title: 'Delegate / Waiting', desc: 'Urgent, not important', cls: 'delegate', shortcut: 'E' },
	eliminate: { title: 'Eliminate / Later', desc: 'Not important, not urgent', cls: 'eliminate', shortcut: 'R' }
};

export interface Task {
	id: string;
	userId: string;
	title: string;
	description: string;
	isImportant: boolean;
	isUrgent: boolean;
	dueDate: number | null;
	reminderAt: number | null;
	isCompleted: boolean;
	isArchived: boolean;
	isPinned: boolean;
	category: string;
	createdAt: number;
	updatedAt: number;
	sync_version: number | null;
	deleted: number;
	encrypted_blob?: string;
}

export interface VaultState {
	userId: string;
	vaultId: string;
	deviceSalt: string;
	validationValue: string;
	createdAt: number;
}

export interface DeviceState {
	deviceId: string;
	userId: string;
	lastSyncAt: number | null;
}

export interface Session {
	id: 'current';
	userId: string;
	key: CryptoKey;
	createdAt: number;
}

export class EisenDB extends Dexie {
	tasks!: Table<Task, string>;
	vaults!: Table<VaultState, string>;
	deviceState!: Table<DeviceState, string>;
	sessions!: Table<Session, 'current'>;

	constructor() {
		super('eisen-web');
		this.version(1).stores({
			tasks: 'id, userId, isCompleted, isArchived, isPinned, dueDate, updatedAt, createdAt, deleted, sync_version',
			vaults: 'userId',
			deviceState: 'deviceId, userId',
			sessions: 'id, userId, createdAt'
		});
	}
}

export const db = new EisenDB();

export function sortTasks(tasks: Task[]): Task[] {
	return [...tasks].sort((a, b) => {
		if (a.isPinned !== b.isPinned) return a.isPinned ? -1 : 1;
		if (a.dueDate !== null && b.dueDate === null) return -1;
		if (a.dueDate === null && b.dueDate !== null) return 1;
		if (a.dueDate !== null && b.dueDate !== null) return a.dueDate - b.dueDate;
		return b.createdAt - a.createdAt;
	});
}

export function liveActiveTasks(userId: string) {
	return liveQuery(() =>
		db.tasks
			.where({ userId, deleted: 0 })
			.toArray()
			.then((list) => sortTasks(list.filter((t) => !t.isArchived && !t.isCompleted)))
	);
}

export function searchTasks(userId: string, query: string) {
	const q = query.trim().toLowerCase();
	return liveQuery(() =>
		db.tasks
			.where({ userId, deleted: 0 })
			.toArray()
			.then((list) =>
				sortTasks(
					list.filter(
						(t) =>
							!t.isArchived &&
							!t.isCompleted &&
							(t.title.toLowerCase().includes(q) ||
								t.description.toLowerCase().includes(q) ||
								t.category.toLowerCase().includes(q))
					)
				)
			)
	);
}

export function liveCompletedTasks(userId: string) {
	return liveQuery(() =>
		db.tasks
			.where({ userId, deleted: 0 })
			.toArray()
			.then((list) =>
				list.filter((t) => t.isCompleted && !t.isArchived).sort((a, b) => b.updatedAt - a.updatedAt)
			)
	);
}

export function liveArchivedTasks(userId: string) {
	return liveQuery(() =>
		db.tasks
			.where({ userId, deleted: 0 })
			.toArray()
			.then((list) => list.filter((t) => t.isArchived).sort((a, b) => b.updatedAt - a.updatedAt))
	);
}

export async function addTask(
	userId: string,
	title: string,
	description: string,
	isImportant: boolean,
	isUrgent: boolean,
	options: { dueDate?: number | null; reminderAt?: number | null; isPinned?: boolean; category?: string } = {}
): Promise<Task> {
	const now = Date.now();
	const task: Task = {
		id: crypto.randomUUID(),
		userId,
		title: title.trim(),
		description: description.trim(),
		isImportant,
		isUrgent,
		dueDate: options.dueDate ?? null,
		reminderAt: options.reminderAt ?? null,
		isCompleted: false,
		isArchived: false,
		isPinned: options.isPinned ?? false,
		category: (options.category ?? '').trim(),
		createdAt: now,
		updatedAt: now,
		sync_version: null,
		deleted: 0
	};
	await db.tasks.add(task);
	return task;
}

export async function updateTask(
	id: string,
	changes: Partial<Omit<Task, 'id' | 'userId' | 'createdAt' | 'updatedAt'>>
) {
	await db.tasks.update(id, { ...changes, updatedAt: Date.now() });
}

export async function toggleCompleted(id: string) {
	const task = await db.tasks.get(id);
	if (!task) return;
	await db.tasks.update(id, { isCompleted: !task.isCompleted, updatedAt: Date.now() });
}

export async function archiveTask(id: string) {
	await db.tasks.update(id, { isArchived: true, isCompleted: false, updatedAt: Date.now() });
}

export async function restoreTask(id: string) {
	await db.tasks.update(id, { isArchived: false, isCompleted: false, updatedAt: Date.now() });
}

export async function togglePin(id: string) {
	const task = await db.tasks.get(id);
	if (!task) return;
	await db.tasks.update(id, { isPinned: !task.isPinned, updatedAt: Date.now() });
}

export async function deleteTask(id: string) {
	await db.tasks.update(id, { deleted: 1, updatedAt: Date.now() });
}

export async function getTask(id: string): Promise<Task | undefined> {
	return db.tasks.get(id);
}

export async function getVault(userId: string): Promise<VaultState | undefined> {
	return db.vaults.get(userId);
}

export async function createVaultRecord(
	userId: string,
	validationValue: string,
	deviceSalt: Uint8Array
): Promise<VaultState> {
	const existing = await getVault(userId);
	if (existing) throw new Error('Vault already exists for this user.');

	const vault: VaultState = {
		userId,
		vaultId: crypto.randomUUID(),
		deviceSalt: btoa(String.fromCharCode(...deviceSalt)),
		validationValue,
		createdAt: Date.now()
	};
	await db.vaults.add(vault);
	await db.deviceState.put({
		deviceId: crypto.randomUUID(),
		userId,
		lastSyncAt: null
	});
	return vault;
}

export async function getDeviceId(userId: string): Promise<string> {
	let state = await db.deviceState.where('userId').equals(userId).first();
	if (!state) {
		state = { deviceId: crypto.randomUUID(), userId, lastSyncAt: null };
		await db.deviceState.add(state);
	}
	return state.deviceId;
}

export async function clearUserData(userId: string): Promise<void> {
	await db.transaction('rw', db.tasks, db.vaults, db.deviceState, db.sessions, async () => {
		await db.tasks.where('userId').equals(userId).delete();
		await db.vaults.where('userId').equals(userId).delete();
		await db.deviceState.where('userId').equals(userId).delete();
		await db.sessions.where('userId').equals(userId).delete();
	});
}
