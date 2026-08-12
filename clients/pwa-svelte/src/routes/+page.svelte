<script lang="ts">
	import { browser } from '$app/environment';
	import { liveQuery } from 'dexie';
	import { masterKey, unlock, createAccount, accountExists } from '$lib/vault';
	import {
		liveActiveTasks,
		searchTasks,
		eisenhowerCategory,
		categoryOrder,
		toggleCompleted,
		archiveTask,
		togglePin,
		type Task,
		type EisenhowerCategory
	} from '$lib/db';
	import { search, syncMessage } from '$lib/stores';
	import { Plus, Pin, PinOff, Trash2 } from '@lucide/svelte';

	let password = $state('');
	let message = $state('');
	let busy = $state(false);
	let hasAccount = $state(false);
	let tasks = $state<Task[]>([]);

	let isCreate = $derived(hasAccount === false);
	let modeLabel = $derived(isCreate ? 'Create account' : 'Unlock');
	let modeText = $derived(
		isCreate
			? 'No account found. Choose a passphrase to create one.'
			: 'Enter your passphrase to unlock your local tasks.'
	);

	if (browser) {
		accountExists().then((exists) => {
			hasAccount = exists;
		});
	}

	$effect(() => {
		if (!$masterKey) return;
		const query = $search.trim();
		const source = query ? searchTasks(query) : liveActiveTasks();
		const sub = source.subscribe((list) => {
			tasks = list;
		});
		return () => sub.unsubscribe();
	});

	async function handleUnlock(event: Event) {
		event.preventDefault();
		if (busy) return;
		message = '';
		busy = true;
		try {
			if (isCreate) {
				await createAccount(password);
				message = 'Account created.';
			} else {
				await unlock(password);
			}
			password = '';
		} catch (e) {
			message = e instanceof Error ? e.message : 'Could not unlock. Check your passphrase.';
		} finally {
			busy = false;
		}
	}

	const categoryLabels: Record<
		EisenhowerCategory,
		{ title: string; desc: string; cls: string; shortcut: string }
	> = {
		do_now: { title: 'Do Now', desc: 'Important & urgent', cls: 'do-now', shortcut: 'Q' },
		schedule: { title: 'Schedule', desc: 'Important, not urgent', cls: 'schedule', shortcut: 'W' },
		delegate: { title: 'Delegate / Waiting', desc: 'Urgent, not important', cls: 'delegate', shortcut: 'E' },
		eliminate: { title: 'Eliminate / Later', desc: 'Not important, not urgent', cls: 'eliminate', shortcut: 'R' }
	};

	function tasksByCategory(cat: EisenhowerCategory) {
		return tasks.filter((t) => eisenhowerCategory(t) === cat);
	}

	function formatDueDate(date: number | null): string {
		if (!date) return '';
		return new Date(date).toLocaleDateString();
	}
</script>

{#if !$masterKey}
	<div class="unlock-screen">
		<h1>Eisen</h1>
		<p>{modeText}</p>
		<form onsubmit={handleUnlock}>
			<input type="password" bind:value={password} placeholder="Passphrase" disabled={busy} />
			<button class="primary" type="submit" disabled={busy}>{busy ? 'Working…' : modeLabel}</button>
		</form>
		{#if message}
			<p class="error">{message}</p>
		{/if}
	</div>
{:else}
	{#if $syncMessage}
		<p class="error">{$syncMessage}</p>
	{/if}

	{#if tasks.length === 0}
		<div class="empty-state">No active tasks. Add one to get started.</div>
	{/if}

	{#each categoryOrder as cat (cat)}
		{@const list = tasksByCategory(cat)}
		{@const info = categoryLabels[cat]}
		<div class="section-header {info.cls}">
			<span>{info.title}</span>
			<span class="badge">{list.length}</span>
		</div>
		{#if list.length === 0}
			<div class="empty-state">{info.desc}</div>
		{:else}
			{#each list as task (task.id)}
				<div class="card">
					<div class="card-row">
						<input
							type="checkbox"
							checked={task.isCompleted}
							onchange={async () => await toggleCompleted(task.id)}
							aria-label="Mark {task.title} complete"
						/>
						<div class="grow">
							<a class="task-title" href="/task/{task.id}">{task.title}</a>
							{#if task.dueDate || task.category}
								<p class="task-meta">
									{formatDueDate(task.dueDate)}{task.dueDate && task.category ? ' · ' : ''}{task.category}
								</p>
							{/if}
						</div>
						<div class="row-actions">
							<button
								class="icon-button"
								onclick={async () => await togglePin(task.id)}
								aria-pressed={task.isPinned}
								aria-label={task.isPinned ? 'Unpin' : 'Pin'}
							>
								{#if task.isPinned}
									<PinOff size={20} />
								{:else}
									<Pin size={20} />
								{/if}
							</button>
							<button
								class="icon-button"
								onclick={async () => await archiveTask(task.id)}
								aria-label="Archive"
							>
								<Trash2 size={20} />
							</button>
						</div>
					</div>
				</div>
			{/each}
		{/if}
	{/each}

	<a class="fab" href="/new-task" aria-label="+ Add">
		<Plus size={20} />
		<span>Add</span>
	</a>
{/if}
