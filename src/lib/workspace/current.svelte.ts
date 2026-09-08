import type { OpenState, Workspace } from './workspace';

let bound: Workspace | null = $state(null);
let tick = $state(0);
let unsub: (() => void) | null = null;

export function bindWorkspace(ws: Workspace | null) {
	if (bound === ws) return;
	unsub?.();
	unsub = null;
	if (bound && bound !== ws) bound.close();
	bound = ws;
	tick += 1;
	if (ws) {
		unsub = ws.subscribe(() => {
			tick += 1;
		});
	}
}

export function workspaceEpoch(): number {
	return tick;
}

export function currentWorkspace(): Workspace | null {
	void tick;
	return bound;
}

export function currentOpen(): OpenState | null {
	void tick;
	if (!bound) return null;
	const state = bound.state;
	return state.status === 'open' ? state : null;
}
