<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { masterKey } from '$lib/vault';
	import { addTask, categoryOrder, type EisenhowerCategory } from '$lib/db';
	import { ArrowLeft } from '@lucide/svelte';

	const categoryInfo: Record<
		EisenhowerCategory,
		{ title: string; desc: string; cls: string }
	> = {
		do_now: { title: 'Do Now', desc: 'Important & urgent', cls: 'do-now' },
		schedule: { title: 'Schedule', desc: 'Important, not urgent', cls: 'schedule' },
		delegate: { title: 'Delegate', desc: 'Urgent, not important', cls: 'delegate' },
		eliminate: { title: 'Eliminate', desc: 'Neither urgent nor important', cls: 'eliminate' }
	};

	const categoryFromString = (s?: string | null): EisenhowerCategory => {
		if (s && categoryOrder.includes(s as EisenhowerCategory)) return s as EisenhowerCategory;
		return 'do_now';
	};

	let selected = $state<EisenhowerCategory>(categoryFromString($page.params.category));
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

	async function handleSubmit(event: Event) {
		event.preventDefault();
		if (!title.trim()) {
			error = 'Title is required';
			return;
		}
		const flags = categoryToFlags(selected);
		const due = dueDate ? new Date(dueDate).getTime() : null;
		const rem = reminderAt ? new Date(reminderAt).getTime() : null;
		if (rem && rem < Date.now()) {
			error = 'Reminder time is in the past';
			return;
		}
		error = '';
		await addTask(title.trim(), description.trim(), flags.isImportant, flags.isUrgent, {
			dueDate: due,
			reminderAt: rem,
			category: categoryTag.trim()
		});
		goto('/');
	}

	function handleBack() {
		if (confirm('Discard this task?')) {
			goto('/');
		}
	}
</script>

{#if !$masterKey}
	<p class="error">Please unlock the app from the home screen.</p>
{:else}
	<form onsubmit={handleSubmit}>
		<div class="composer-header">
			<button type="button" class="icon-button" onclick={handleBack} aria-label="Discard and go back">
				<ArrowLeft size={24} />
			</button>
			<h2>New task</h2>
			<button class="primary" type="submit">Save</button>
		</div>

		{#if error}
			<p class="error">{error}</p>
		{/if}

		<input type="text" placeholder="Title" bind:value={title} />

		<div class="category-grid">
			{#each categoryOrder as cat (cat)}
				{@const info = categoryInfo[cat]}
				<button
					type="button"
					class="category-cell {info.cls}"
					class:selected={selected === cat}
					onclick={() => (selected = cat)}
				>
					<strong>{info.title}</strong>
					<span>{info.desc}</span>
				</button>
			{/each}
		</div>

		<textarea placeholder="Notes" bind:value={description} rows="4"></textarea>

		<input type="text" placeholder="Category tag" bind:value={categoryTag} />

		<div class="metadata-row">
			<label>
				Due date
				<input type="date" bind:value={dueDate} />
			</label>
		</div>

		<div class="metadata-row">
			<label>
				Reminder
				<input type="datetime-local" bind:value={reminderAt} />
			</label>
		</div>
	</form>
{/if}
