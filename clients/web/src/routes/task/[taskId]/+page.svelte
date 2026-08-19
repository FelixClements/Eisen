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
	import { masterKey } from '$lib/vault';
	import {
		getTask,
		updateTask,
		toggleCompleted,
		togglePin,
		archiveTask,
		restoreTask,
		eisenhowerCategory,
		categoryOrder,
		categoryLabels,
		type EisenhowerCategory
	} from '$lib/db';
	import { scheduleNextWake } from '$lib/notifications';

	let { data } = $props();
	const userId = $derived(data.user?.id ?? '');

	let taskId = $state('');
	let title = $state('');
	let description = $state('');
	let categoryTag = $state('');
	let selected = $state<EisenhowerCategory>('do_now');
	let isCompleted = $state(false);
	let isArchived = $state(false);
	let isPinned = $state(false);
	let due = $state('');
	let reminder = $state('');
	let error = $state('');
	let found = $state(true);

	$effect(() => {
		if (!$masterKey) return;
		const id = $page.params.taskId ?? '';
		if (!id) {
			goto('/');
			return;
		}
		taskId = id;
		getTask(id).then((t) => {
			if (!t || t.userId !== userId) {
				found = false;
				goto('/');
				return;
			}
			found = true;
			title = t.title;
			description = t.description;
			categoryTag = t.category;
			selected = eisenhowerCategory(t);
			isCompleted = t.isCompleted;
			isArchived = t.isArchived;
			isPinned = t.isPinned;
			due = t.dueDate ? new Date(t.dueDate).toISOString().slice(0, 10) : '';
			reminder = t.reminderAt ? new Date(t.reminderAt).toISOString().slice(0, 16) : '';
		});
	});

	function flagsFromCategory(cat: EisenhowerCategory) {
		return {
			isImportant: cat === 'do_now' || cat === 'schedule',
			isUrgent: cat === 'do_now' || cat === 'delegate'
		};
	}

	async function setCategory(cat: EisenhowerCategory) {
		selected = cat;
		await updateTask(taskId, flagsFromCategory(cat));
	}

	async function saveAndSchedule() {
		if (userId) await scheduleNextWake(userId);
	}
</script>

{#if $masterKey && found}
	<Page>
		<Navbar title={title || 'Task'}>
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
				onBlur={async () => {
					await updateTask(taskId, { title });
					await saveAndSchedule();
				}}
			/>
		</List>

		<Block strong inset>
			<div class="grid grid-cols-2 gap-2">
				{#each categoryOrder as cat (cat)}
					{@const info = categoryLabels[cat]}
					<button
						type="button"
						class="rounded-xl border p-2 text-left text-sm {info.cls}"
						class:ring-2={selected === cat}
						onclick={() => setCategory(cat)}
					>
						{info.title}
					</button>
				{/each}
			</div>
		</Block>

		<List strongIos outlineIos>
			<ListInput
				label="Notes"
				type="textarea"
				bind:value={description}
				onBlur={async () => updateTask(taskId, { description })}
			/>
			<ListInput
				label="Category"
				type="text"
				bind:value={categoryTag}
				onBlur={async () => updateTask(taskId, { category: categoryTag })}
			/>
			<ListInput
				label="Due date"
				type="date"
				bind:value={due}
				onChange={async () =>
					updateTask(taskId, { dueDate: due ? new Date(due).getTime() : null })}
			/>
			<ListInput
				label="Reminder"
				type="datetime-local"
				bind:value={reminder}
				onChange={async () => {
					const time = reminder ? new Date(reminder).getTime() : null;
					if (time && time < Date.now()) {
						error = 'Reminder is in the past';
						return;
					}
					error = '';
					await updateTask(taskId, { reminderAt: time });
					await saveAndSchedule();
				}}
			/>
			<ListItem title="Completed">
				{#snippet after()}
					<Toggle
						checked={isCompleted}
						onChange={async () => {
							await toggleCompleted(taskId);
							isCompleted = !isCompleted;
						}}
					/>
				{/snippet}
			</ListItem>
			<ListItem title="Pinned">
				{#snippet after()}
					<Toggle
						checked={isPinned}
						onChange={async () => {
							await togglePin(taskId);
							isPinned = !isPinned;
						}}
					/>
				{/snippet}
			</ListItem>
		</List>

		<Block strong inset class="flex gap-2">
			{#if isArchived}
				<Button
					outline
					onclick={async () => {
						await restoreTask(taskId);
						isArchived = false;
					}}>Restore</Button
				>
			{:else}
				<Button
					outline
					onclick={async () => {
						await archiveTask(taskId);
						isArchived = true;
					}}>Archive</Button
				>
			{/if}
		</Block>
	</Page>
{:else}
	<Page>
		<Block strong inset><p>Unlock your vault to view this task.</p></Block>
	</Page>
{/if}
