import Dexie, { liveQuery, type Table } from 'dexie';

export type Quadrant = 'ui' | 'un' | 'in' | 'nn';

export interface Todo {
	id: string;
	title: string;
	notes: string;
	completed: boolean;
	quadrant: Quadrant;
	local_updated_at: number;
	sync_version: number | null;
	deleted: number;
	encrypted_blob?: string;
}

export class EisenDB extends Dexie {
	todos!: Table<Todo, string>;

	constructor() {
		super('eisen-db');
		this.version(1).stores({
			todos: 'id, completed, local_updated_at, sync_version, deleted'
		});
	}
}

export const db = new EisenDB();

export async function addTodo(title: string, notes: string, quadrant: Quadrant): Promise<Todo> {
	const todo: Todo = {
		id: crypto.randomUUID(),
		title,
		notes,
		completed: false,
		quadrant,
		local_updated_at: Date.now(),
		sync_version: null,
		deleted: 0
	};
	await db.todos.add(todo);
	return todo;
}

export async function updateTodo(id: string, changes: Partial<Omit<Todo, 'id' | 'local_updated_at'>>): Promise<void> {
	await db.todos.update(id, { ...changes, local_updated_at: Date.now() });
}

export async function toggleTodo(id: string): Promise<void> {
	const todo = await db.todos.get(id);
	if (!todo) return;
	await db.todos.update(id, { completed: !todo.completed, local_updated_at: Date.now() });
}

export async function deleteTodo(id: string): Promise<void> {
	await db.todos.update(id, { deleted: 1, local_updated_at: Date.now() });
}

export function liveTodos() {
	return liveQuery(() => db.todos.where('deleted').notEqual(1).sortBy('local_updated_at'));
}
