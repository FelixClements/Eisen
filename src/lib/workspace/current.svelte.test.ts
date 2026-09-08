import { describe, expect, it, vi } from 'vitest';
import {
	bindWorkspace,
	currentWorkspace,
	workspaceEpoch
} from './current.svelte';
import type { Workspace } from './workspace';

function fakeWorkspace(): Workspace & { close: ReturnType<typeof vi.fn> } {
	const close = vi.fn();
	return {
		ready: Promise.resolve(),
		get state() {
			return { status: 'booting' } as const;
		},
		subscribe: () => () => {},
		close
	};
}

describe('bindWorkspace', () => {
	it('does not bump the epoch when already unbound', () => {
		bindWorkspace(null);
		const epoch = workspaceEpoch();
		bindWorkspace(null);
		expect(currentWorkspace()).toBeNull();
		expect(workspaceEpoch()).toBe(epoch);
	});

	it('closes the previous workspace once when unbound', () => {
		const ws = fakeWorkspace();
		bindWorkspace(ws);
		const afterBind = workspaceEpoch();
		expect(afterBind).toBeGreaterThan(0);
		bindWorkspace(null);
		expect(ws.close).toHaveBeenCalledOnce();
		expect(currentWorkspace()).toBeNull();
		expect(workspaceEpoch()).toBeGreaterThan(afterBind);
		bindWorkspace(null);
		expect(ws.close).toHaveBeenCalledOnce();
	});
});
