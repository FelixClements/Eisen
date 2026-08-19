<script lang="ts">
	import { goto } from '$app/navigation';
	import { Page, Navbar, NavbarBackLink, Block, List, ListInput, Button } from 'konsta/svelte';
	import { masterKey, vaultUserId } from '$lib/vault';
	import { addTask, categoryOrder, categoryLabels, type EisenhowerCategory } from '$lib/db';
	import { scheduleNextWake } from '$lib/notifications';

	let { data } = $props();
	const userId = $derived(data.user?.id ?? '');

	let selected = $state<EisenhowerCategory>('do_now');
	let title = $state('');
	let description = $state('');
	let categoryTag = $state('');
	let dueDate = $state('');
	let reminderAt = $state('');
	let error = $state('');

	function categoryToFlags(cat: EisenhowerCategory) {
		return {
			isImportant: cat === 'do_now' || cat === 'schedule',
			isUrgent: cat === 'do_now' || cat === 'delegate'
		};
	}

	async function handleSubmit(e: Event) {
		e.preventDefault();
		if (!$masterKey || !userId) {
			error = 'Unlock your vault first.';
			return;
		}
		if (!title.trim()) {
			error = 'Title is required';
			return;
		}
		const due = dueDate ? new Date(dueDate).getTime() : null;
		const rem = reminderAt ? new Date(reminderAt).getTime() : null;
		if (rem && rem < Date.now()) {
			error = 'Reminder time is in the past';
			return;
		}
		const flags = categoryToFlags(selected);
		await addTask(userId, title.trim(), description.trim(), flags.isImportant, flags.isUrgent, {
			dueDate: due,
			reminderAt: rem,
			category: categoryTag.trim()
		});
		await scheduleNextWake(userId);
		goto('/');
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

	<Block strong inset class="space-y-4">
		{#if error}
			<p class="text-red-600">{error}</p>
		{/if}
		<List strongIos outlineIos>
			<ListInput label="Title" type="text" placeholder="What needs doing?" bind:value={title} />
		</List>

		<div class="grid grid-cols-2 gap-2">
			{#each categoryOrder as cat (cat)}
				{@const info = categoryLabels[cat]}
				<button
					type="button"
					class="rounded-xl border p-3 text-left {info.cls}"
					class:ring-2={selected === cat}
					class:ring-primary={selected === cat}
					onclick={() => (selected = cat)}
				>
					<strong class="block">{info.title}</strong>
					<span class="text-sm opacity-80">{info.desc}</span>
				</button>
			{/each}
		</div>

		<List strongIos outlineIos>
			<ListInput label="Notes" type="textarea" placeholder="Details…" bind:value={description} />
			<ListInput label="Category tag" type="text" bind:value={categoryTag} />
			<ListInput label="Due date" type="date" bind:value={dueDate} />
			<ListInput label="Reminder" type="datetime-local" bind:value={reminderAt} />
		</List>
	</Block>
</Page>
