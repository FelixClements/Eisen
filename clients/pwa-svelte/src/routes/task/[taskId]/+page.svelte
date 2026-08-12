<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
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
		type Task,
		type EisenhowerCategory
	} from '$lib/db';

	const categoryInfo: Record<
		EisenhowerCategory,
		{ title: string; desc: string; cls: string }
	> = {
		do_now: { title: 'Do Now', desc: 'Important & urgent', cls: 'do-now' },
		schedule: { title: 'Schedule', desc: 'Important, not urgent', cls: 'schedule' },
		delegate: { title: 'Delegate', desc: 'Urgent, not important', cls: 'delegate' },
		eliminate: { title: 'Eliminate', desc: 'Neither urgent nor important', cls: 'eliminate' }
	};

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
			if (!t) {
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

	function setCategory(cat: EisenhowerCategory) {
		selected = cat;
		updateTask(taskId, flagsFromCategory(cat));
	}

	function updateTitle(value: string) {
		title = value;
		updateTask(taskId, { title: value });
	}

	function updateDescription(value: string) {
		description = value;
		updateTask(taskId, { description: value });
	}

	function updateCategoryTag(value: string) {
		categoryTag = value;
		updateTask(taskId, { category: value });
	}

	function updateDue() {
		updateTask(taskId, { dueDate: due ? new Date(due).getTime() : null });
	}

	function updateReminder() {
		const time = reminder ? new Date(reminder).getTime() : null;
		if (time && time < Date.now()) {
			error = 'Reminder time is in the past';
			return;
		}
		error = '';
		updateTask(taskId, { reminderAt: time });
	}
</script>

{#if !$masterKey}
	<p class="error">Please unlock the app from the home screen.</p>
{:else if !found}
	<p>Task not found.</p>
{:else}
	<div class="composer-header">
		<button class="icon-button" onclick={() => goto('/')}>←</button>
		<h2>{title || 'Task detail'}</h2>
	</div>

	{#if error}
		<p class="error">{error}</p>
	{/if}

	<input
		type="text"
		value={title}
		oninput={(e) => updateTitle(e.currentTarget.value)}
		placeholder="Title"
	/>

	<div class="category-grid">
		{#each categoryOrder as cat (cat)}
			{@const info = categoryInfo[cat]}
			<button
				type="button"
				class="category-cell {info.cls}"
				class:selected={selected === cat}
				onclick={() => setCategory(cat)}
			>
				<strong>{info.title}</strong>
				<span>{info.desc}</span>
			</button>
		{/each}
	</div>

	<textarea
		value={description}
		oninput={(e) => updateDescription(e.currentTarget.value)}
		placeholder="Notes"
		rows="4"
	></textarea>

	<input
		type="text"
		value={categoryTag}
		oninput={(e) => updateCategoryTag(e.currentTarget.value)}
		placeholder="Category tag"
	/>

	<div class="metadata-row">
		<label>
			Due date
			<input type="date" bind:value={due} onchange={updateDue} />
		</label>
	</div>

	<div class="metadata-row">
		<label>
			Reminder
			<input type="datetime-local" bind:value={reminder} onchange={updateReminder} />
		</label>
	</div>

	<div class="row-actions">
		<button
			onclick={async () => {
				await toggleCompleted(taskId);
				isCompleted = !isCompleted;
			}}
		>
			{isCompleted ? 'Mark active' : 'Mark complete'}
		</button>
		<button
			onclick={async () => {
				await togglePin(taskId);
				isPinned = !isPinned;
			}}
		>
			{isPinned ? 'Unpin' : 'Pin'}
		</button>
		{#if isArchived}
			<button
				onclick={async () => {
					await restoreTask(taskId);
					isArchived = false;
				}}>Restore</button
			>
		{:else}
			<button
				onclick={async () => {
					await archiveTask(taskId);
					isArchived = true;
				}}>Archive</button
			>
		{/if}
	</div>
{/if}
