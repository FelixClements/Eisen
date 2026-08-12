<script lang="ts">
	import { liveQuery } from 'dexie';
	import { masterKey, unlock } from '$lib/vault';
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
	import { sync } from '$lib/sync';

	let password = $state('');
	let message = $state('');
	let search = $state('');
	let searchActive = $state(false);
	let tasks = $state<Task[]>([]);
	let syncing = $state(false);

	$effect(() => {
		if (!$masterKey) return;
		const source = searchActive && search.trim() ? searchTasks(search.trim()) : liveActiveTasks();
		const sub = source.subscribe((list) => {
			tasks = list;
		});
		return () => sub.unsubscribe();
	});

	async function handleUnlock(event: Event) {
		event.preventDefault();
		message = '';
		try {
			await unlock(password);
			password = '';
		} catch (e) {
			message = 'Could not unlock. Check your passphrase.';
		}
	}

	async function handleSync() {
		if (!$masterKey) return;
		syncing = true;
		message = '';
		try {
			await sync($masterKey);
		} catch (e) {
			message = e instanceof Error ? e.message : 'Sync failed';
		} finally {
			syncing = false;
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
		<p>Enter your passphrase to unlock your local tasks.</p>
		<form onsubmit={handleUnlock}>
			<input type="password" bind:value={password} placeholder="Passphrase" />
			<button class="primary" type="submit">Unlock</button>
		</form>
		{#if message}
			<p class="error">{message}</p>
		{/if}
	</div>
{:else}
	<div class="home-actions">
		{#if searchActive}
			<div class="search-bar">
				<input type="text" bind:value={search} placeholder="Search tasks…" autofocus />
				<button class="icon-button" onclick={() => (searchActive = false)}>×</button>
			</div>
		{:else}
			<div class="home-actions-row">
				<button class="icon-button" onclick={() => (searchActive = true)} aria-label="Search">🔍</button>
				<button onclick={handleSync} disabled={syncing}>{syncing ? 'Syncing…' : 'Sync'}</button>
			</div>
		{/if}
	</div>

	{#if message}
		<p class="error">{message}</p>
	{/if}

	{#if tasks.length === 0}
		<div class="empty-state">No active tasks. Add one to get started.</div>
	{:else}
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
								onchange={() => toggleCompleted(task.id)}
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
								<button class="icon-button" onclick={() => togglePin(task.id)} aria-label="Pin">
									{task.isPinned ? '📌' : 'Pin'}
								</button>
								<button class="icon-button" onclick={() => archiveTask(task.id)} aria-label="Archive">🗑</button>
							</div>
						</div>
					</div>
				{/each}
			{/if}
		{/each}
	{/if}

	<a class="fab primary" href="/new-task">+ Add</a>
{/if}
