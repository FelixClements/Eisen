import { EisenErrorException, type TaskEdit, type TaskId, type TaskView } from './types';
import { quadrantOf, utf8Bytes } from './matrix';
import type { CatalogTask } from './task-codec';
import type { TaskCodec } from './task-codec';
import type { Clock } from './ports';

const TITLE_MAX = 256;
const NOTES_MAX = 4096;

function utf8TooLong(value: string, max: number): boolean {
	return utf8Bytes(value) > max;
}

export type TaskCatalog = {
	apply(edit: TaskEdit): Promise<TaskId>;
	views(): TaskView[];
	get(id: TaskId): TaskView | undefined;
	find(id: TaskId): CatalogTask | undefined;
	all(): Iterable<CatalogTask>;
	clear(): void;
	set(task: CatalogTask): void;
};

export function createTaskCatalog(opts: {
	codec: TaskCodec;
	clock: Clock;
	getDeviceId: () => string;
	isOpen: () => boolean;
	persist: (task: CatalogTask) => Promise<void>;
	onChanged: (taskId: TaskId) => void;
}): TaskCatalog {
	const tasks = new Map<string, CatalogTask>();

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

	function requireTask(id: string): CatalogTask {
		const task = tasks.get(id);
		if (!task || task.deleted) throw new EisenErrorException({ code: 'task-missing' });
		return task;
	}

	return {
		async apply(edit) {
			if (!opts.isOpen()) {
				throw new EisenErrorException({ code: 'storage', message: 'Workspace is not open' });
			}
			const now = opts.clock.now();
			const deviceId = opts.getDeviceId();

			if (edit.kind === 'create') {
				if (edit.remindAt && edit.remindAt < now) {
					throw new EisenErrorException({ code: 'invalid-task', reason: 'Reminder is in the past' });
				}
				const id = opts.clock.uuid();
				const task: CatalogTask = {
					id,
					title: validateTitle(edit.title),
					notes: validateNotes(edit.notes ?? ''),
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
				await opts.persist(task);
				opts.onChanged(id);
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
			await opts.persist(task);
			opts.onChanged(task.id);
			return task.id;
		},

		views() {
			return [...tasks.values()].filter((t) => !t.deleted).map((t) => opts.codec.toView(t));
		},

		get(id) {
			return this.views().find((t) => t.id === id);
		},

		find(id) {
			const task = tasks.get(id);
			if (!task || task.deleted) return undefined;
			return task;
		},

		all() {
			return tasks.values();
		},

		clear() {
			tasks.clear();
		},

		set(task) {
			tasks.set(task.id, task);
		}
	};
}
