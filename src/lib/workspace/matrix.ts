import type { Matrix, Quadrant, TaskView } from './types';

export const QUADRANT_ORDER: readonly Quadrant[] = ['do-now', 'schedule', 'delegate', 'eliminate'];

export const QUADRANT_META: Record<
	Quadrant,
	{ label: string; hint: string; shortcut: string; cls: string }
> = {
	'do-now': { label: 'Do Now', hint: 'Important & urgent', shortcut: 'Q', cls: 'do-now' },
	schedule: { label: 'Schedule', hint: 'Important, not urgent', shortcut: 'W', cls: 'schedule' },
	delegate: {
		label: 'Delegate / Waiting',
		hint: 'Urgent, not important',
		shortcut: 'E',
		cls: 'delegate'
	},
	eliminate: {
		label: 'Eliminate / Later',
		hint: 'Not important, not urgent',
		shortcut: 'R',
		cls: 'eliminate'
	}
};

export function quadrantOf(task: { important: boolean; urgent: boolean }): Quadrant {
	if (task.important && task.urgent) return 'do-now';
	if (task.important && !task.urgent) return 'schedule';
	if (!task.important && task.urgent) return 'delegate';
	return 'eliminate';
}

export function flagsFromQuadrant(quadrant: Quadrant): { important: boolean; urgent: boolean } {
	return {
		important: quadrant === 'do-now' || quadrant === 'schedule',
		urgent: quadrant === 'do-now' || quadrant === 'delegate'
	};
}

export function utf8Bytes(s: string): number {
	return new TextEncoder().encode(s).length;
}

export function sortTasks(tasks: TaskView[]): TaskView[] {
	return [...tasks].sort((a, b) => {
		if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
		if (a.dueAt !== null && b.dueAt === null) return -1;
		if (a.dueAt === null && b.dueAt !== null) return 1;
		if (a.dueAt !== null && b.dueAt !== null && a.dueAt !== b.dueAt) return a.dueAt - b.dueAt;
		return b.createdAt - a.createdAt;
	});
}

function matchesFilter(task: TaskView, filter: string): boolean {
	const q = filter.trim().toLowerCase();
	if (!q) return true;
	return (
		task.title.toLowerCase().includes(q) ||
		task.notes.toLowerCase().includes(q) ||
		task.tag.toLowerCase().includes(q)
	);
}

export function buildMatrix(tasks: TaskView[], filter: string): Matrix {
	const active = sortTasks(
		tasks.filter((t) => !t.completed && !t.archived && matchesFilter(t, filter))
	);
	const cells = Object.fromEntries(
		QUADRANT_ORDER.map((q) => {
			const meta = QUADRANT_META[q];
			return [
				q,
				{
					label: meta.label,
					hint: meta.hint,
					shortcut: meta.shortcut,
					tasks: active.filter((t) => t.quadrant === q)
				}
			];
		})
	) as Record<Quadrant, Matrix['cells'][Quadrant]>;
	return { order: QUADRANT_ORDER, cells, total: active.length };
}
