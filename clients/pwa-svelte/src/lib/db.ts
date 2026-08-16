import Dexie, { liveQuery, type Table } from 'dexie';

export type EisenhowerCategory = 'do_now' | 'schedule' | 'delegate' | 'eliminate';

export function eisenhowerCategory(task: {
	isImportant: boolean;
	isUrgent: boolean;
}): EisenhowerCategory {
	if (task.isImportant && task.isUrgent) return 'do_now';
	if (task.isImportant && !task.isUrgent) return 'schedule';
	if (!task.isImportant && task.isUrgent) return 'delegate';
	return 'eliminate';
}

export const categoryOrder: EisenhowerCategory[] = ['do_now', 'schedule', 'delegate', 'eliminate'];

export interface Task {
	id: string;
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

export interface Account {
	ownerId: string;
	vaultId: string;
	deviceSalt: string;
	validationValue: string;
	createdAt: number;
}

export interface DeviceState {
	deviceId: string;
	ownerId: string;
	lastSyncAt: number | null;
}

export interface Session {
	id: 'current';
	key: CryptoKey;
	createdAt: number;
}

export class EisenDB extends Dexie {
	tasks!: Table<Task, string>;
	accounts!: Table<Account, string>;
	deviceState!: Table<DeviceState, string>;
	sessions!: Table<Session, 'current'>;

	constructor() {
		super('eisen-pwa');
		this.version(3).stores({
			tasks:
				'id, isCompleted, isArchived, isPinned, dueDate, updatedAt, createdAt, deleted, sync_version',
			accounts: 'ownerId',
			deviceState: 'deviceId, ownerId',
			sessions: 'id, createdAt'
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

function activeFilter(t: Task) {
	return !t.isArchived && !t.isCompleted && !t.deleted;
}

export function liveActiveTasks() {
	return liveQuery(() =>
		db.tasks
			.where('deleted')
			.equals(0)
			.toArray()
			.then((list) => sortTasks(list.filter((t) => !t.isArchived && !t.isCompleted)))
	);
}

export function searchTasks(query: string) {
	const q = query.trim().toLowerCase();
	return liveQuery(() =>
		db.tasks
			.where('deleted')
			.equals(0)
			.toArray()
			.then((list) =>
				sortTasks(
					list
						.filter(
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

export function liveCompletedTasks() {
	return liveQuery(() =>
		db.tasks
			.where('deleted')
			.equals(0)
			.toArray()
			.then((list) =>
				list
					.filter((t) => t.isCompleted && !t.isArchived)
					.sort((a, b) => b.updatedAt - a.updatedAt)
			)
	);
}

export function liveArchivedTasks() {
	return liveQuery(() =>
		db.tasks
			.where('deleted')
			.equals(0)
			.toArray()
			.then((list) =>
				list
					.filter((t) => t.isArchived)
					.sort((a, b) => b.updatedAt - a.updatedAt)
			)
	);
}

export async function addTask(
	title: string,
	description: string,
	isImportant: boolean,
	isUrgent: boolean,
	options: { dueDate?: number | null; reminderAt?: number | null; isPinned?: boolean; category?: string } = {}
): Promise<Task> {
	const now = Date.now();
	const task: Task = {
		id: crypto.randomUUID(),
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

export async function updateTask(id: string, changes: Partial<Omit<Task, 'id' | 'createdAt' | 'updatedAt'>>) {
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

export async function unarchiveCompleted(id: string) {
	await db.tasks.update(id, { isArchived: false, updatedAt: Date.now() });
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

export async function getAccount(): Promise<Account | undefined> {
	return db.accounts.toCollection().first();
}

export async function clearAllData(): Promise<void> {
	await db.delete();
	if (typeof localStorage !== 'undefined') {
		localStorage.clear();
	}
}

export async function createAccountRecord(
	validationValue: string,
	deviceSalt: Uint8Array
): Promise<Account> {
	const existing = await getAccount();
	if (existing) {
		throw new Error('An account already exists on this device.');
	}
	const account: Account = {
		ownerId: crypto.randomUUID(),
		vaultId: crypto.randomUUID(),
		deviceSalt: btoa(String.fromCharCode(...deviceSalt)),
		validationValue,
		createdAt: Date.now()
	};
	await db.accounts.add(account);
	await db.deviceState.add({
		deviceId: crypto.randomUUID(),
		ownerId: account.ownerId,
		lastSyncAt: null
	});
	return account;
}
