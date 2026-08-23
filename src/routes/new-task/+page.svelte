<script lang="ts">
	import { goto } from '$app/navigation';
	import { Page, Navbar, NavbarBackLink, Block, List, ListInput, Button } from 'konsta/svelte';
	import { currentOpen } from '$lib/workspace/current.svelte';
	import { QUADRANT_ORDER, QUADRANT_META, flagsFromQuadrant, type Quadrant } from '$lib/workspace';
	import { EisenErrorException } from '$lib/workspace/types';

	let selected = $state<Quadrant>('do-now');
	let title = $state('');
	let notes = $state('');
	let tag = $state('');
	let dueDate = $state('');
	let reminderAt = $state('');
	let error = $state('');

	async function handleSubmit(e: Event) {
		e.preventDefault();
		const open = currentOpen();
		if (!open) {
			error = 'Sign in first.';
			return;
		}
		const flags = flagsFromQuadrant(selected);
		try {
			await open.apply({
				kind: 'create',
				title,
				notes,
				tag,
				important: flags.important,
				urgent: flags.urgent,
				dueAt: dueDate ? new Date(dueDate).getTime() : null,
				remindAt: reminderAt ? new Date(reminderAt).getTime() : null
			});
			goto('/');
		} catch (err) {
			if (err instanceof EisenErrorException && err.error.code === 'invalid-task') {
				error = err.error.reason;
			} else {
				error = err instanceof Error ? err.message : 'Could not save';
			}
		}
	}
</script>

<Page>
	<Navbar title="New task">
		{#snippet left()}
			<NavbarBackLink onclick={() => goto('/')} />
		{/snippet}
		{#snippet right()}
			<Button clear small onclick={handleSubmit}>Save</Button>
		{/snippet}
	</Navbar>

	{#if error}
		<Block strong inset><p class="text-red-600">{error}</p></Block>
	{/if}

	<List strongIos outlineIos>
		<ListInput label="Title" type="text" placeholder="What needs doing?" bind:value={title} />
	</List>

	<Block strong inset>
		<div class="grid grid-cols-2 gap-2">
			{#each QUADRANT_ORDER as cat (cat)}
				{@const info = QUADRANT_META[cat]}
				<button
					type="button"
					class="rounded-xl border p-3 text-left {info.cls}"
					class:ring-2={selected === cat}
					onclick={() => (selected = cat)}
				>
					<strong class="block">{info.label}</strong>
					<span class="text-sm opacity-80">{info.hint}</span>
				</button>
			{/each}
		</div>
	</Block>

	<List strongIos outlineIos>
		<ListInput label="Notes" type="textarea" placeholder="Details…" bind:value={notes} />
		<ListInput label="Category tag" type="text" bind:value={tag} />
		<ListInput label="Due date" type="date" bind:value={dueDate} />
		<ListInput label="Reminder" type="datetime-local" bind:value={reminderAt} />
	</List>
</Page>
