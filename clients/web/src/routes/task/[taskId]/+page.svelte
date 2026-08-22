<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import {
		Page,
		Navbar,
		NavbarBackLink,
		Block,
		List,
		ListInput,
		ListItem,
		Button,
		Toggle
	} from 'konsta/svelte';
	import { currentOpen } from '$lib/workspace/current.svelte';
	import {
		QUADRANT_ORDER,
		QUADRANT_META,
		flagsFromQuadrant,
		type Quadrant
	} from '$lib/workspace';
	import { EisenErrorException } from '$lib/workspace/types';

	const open = $derived(currentOpen());
	const task = $derived(open?.get($page.params.taskId ?? ''));

	let title = $state('');
	let notes = $state('');
	let tag = $state('');
	let selected = $state<Quadrant>('do-now');
	let due = $state('');
	let reminder = $state('');
	let error = $state('');
	let loadedId = $state('');

	$effect(() => {
		if (!task || task.id === loadedId) return;
		loadedId = task.id;
		title = task.title;
		notes = task.notes;
		tag = task.tag;
		selected = task.quadrant;
		due = task.dueAt ? new Date(task.dueAt).toISOString().slice(0, 10) : '';
		reminder = task.remindAt ? new Date(task.remindAt).toISOString().slice(0, 16) : '';
	});

	async function patch(p: Parameters<typeof applyPatch>[0]) {
		await applyPatch(p);
	}

	async function applyPatch(
		p: Partial<{
			title: string;
			notes: string;
			tag: string;
			important: boolean;
			urgent: boolean;
			dueAt: number | null;
			remindAt: number | null;
			pinned: boolean;
		}>
	) {
		if (!open || !task) return;
		try {
			await open.apply({ kind: 'update', id: task.id, patch: p });
			error = '';
		} catch (err) {
			if (err instanceof EisenErrorException && err.error.code === 'invalid-task') {
				error = err.error.reason;
			} else {
				error = err instanceof Error ? err.message : 'Save failed';
			}
		}
	}
</script>

{#if open && task}
	<Page>
		<Navbar title={task.title || 'Task'}>
			{#snippet left()}
				<NavbarBackLink onclick={() => goto('/')} />
			{/snippet}
		</Navbar>

		{#if error}
			<Block strong inset><p class="text-red-600">{error}</p></Block>
		{/if}

		<List strongIos outlineIos>
			<ListInput
				label="Title"
				type="text"
				bind:value={title}
				onBlur={() => patch({ title })}
			/>
		</List>

		<Block strong inset>
			<div class="grid grid-cols-2 gap-2">
				{#each QUADRANT_ORDER as cat (cat)}
					{@const info = QUADRANT_META[cat]}
					<button
						type="button"
						class="rounded-xl border p-2 text-left text-sm {info.cls}"
						class:ring-2={selected === cat}
						onclick={() => {
							selected = cat;
							const flags = flagsFromQuadrant(cat);
							patch(flags);
						}}
					>
						{info.label}
					</button>
				{/each}
			</div>
		</Block>

		<List strongIos outlineIos>
			<ListInput
				label="Notes"
				type="textarea"
				bind:value={notes}
				onBlur={() => patch({ notes })}
			/>
			<ListInput label="Category" type="text" bind:value={tag} onBlur={() => patch({ tag })} />
			<ListInput
				label="Due date"
				type="date"
				bind:value={due}
				onChange={() => patch({ dueAt: due ? new Date(due).getTime() : null })}
			/>
			<ListInput
				label="Reminder"
				type="datetime-local"
				bind:value={reminder}
				onChange={() => patch({ remindAt: reminder ? new Date(reminder).getTime() : null })}
			/>
			<ListItem title="Completed">
				{#snippet after()}
					<Toggle
						checked={task.completed}
						onChange={() => open.apply({ kind: 'complete', id: task.id, done: !task.completed })}
					/>
				{/snippet}
			</ListItem>
			<ListItem title="Pinned">
				{#snippet after()}
					<Toggle
						checked={task.pinned}
						onChange={() => patch({ pinned: !task.pinned })}
					/>
				{/snippet}
			</ListItem>
		</List>

		<Block strong inset class="flex gap-2">
			{#if task.archived}
				<Button outline onclick={() => open.apply({ kind: 'archive', id: task.id, archived: false })}
					>Restore</Button
				>
			{:else}
				<Button outline onclick={() => open.apply({ kind: 'archive', id: task.id, archived: true })}
					>Archive</Button
				>
			{/if}
		</Block>
	</Page>
{:else}
	<Page>
		<Block strong inset><p>Task not found.</p></Block>
	</Page>
{/if}
