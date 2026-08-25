import { describe, expect, it } from 'vitest';
import { createTaskCodec } from './task-codec';
import { createTaskCatalog } from './task-catalog';
import { deriveVaultKey, newKdfSalt } from '$lib/crypto';
import { EisenErrorException } from './types';

const ITER = 1;

async function codec() {
	const salt = await newKdfSalt();
	const key = await deriveVaultKey('password', salt, ITER);
	return createTaskCodec(key);
}

describe('TaskCatalog', () => {
	it('creates a task with validation', async () => {
		const c = await codec();
		const persisted: string[] = [];
		const catalog = createTaskCatalog({
			codec: c,
			clock: { now: () => 1000, uuid: () => 'id-1' },
			getDeviceId: () => 'dev-1',
			isOpen: () => true,
			persist: async (t) => {
				persisted.push(t.id);
			},
			onChanged: () => {}
		});
		const id = await catalog.apply({
			kind: 'create',
			title: 'Hello',
			important: true,
			urgent: false
		});
		expect(id).toBe('id-1');
		expect(persisted).toEqual(['id-1']);
		expect(catalog.views()[0]?.title).toBe('Hello');
	});

	it('rejects empty title', async () => {
		const c = await codec();
		const catalog = createTaskCatalog({
			codec: c,
			clock: { now: () => 1000, uuid: () => 'id-1' },
			getDeviceId: () => 'dev-1',
			isOpen: () => true,
			persist: async () => {},
			onChanged: () => {}
		});
		await expect(
			catalog.apply({ kind: 'create', title: '   ', important: false, urgent: false })
		).rejects.toBeInstanceOf(EisenErrorException);
	});

	it('completes a task', async () => {
		const c = await codec();
		const catalog = createTaskCatalog({
			codec: c,
			clock: { now: () => 1000, uuid: () => 'id-1' },
			getDeviceId: () => 'dev-1',
			isOpen: () => true,
			persist: async () => {},
			onChanged: () => {}
		});
		const id = await catalog.apply({
			kind: 'create',
			title: 'Done me',
			important: true,
			urgent: false
		});
		await catalog.apply({ kind: 'complete', id, done: true });
		expect(catalog.get(id)?.completed).toBe(true);
	});
});
