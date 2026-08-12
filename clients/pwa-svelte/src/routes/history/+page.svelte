<script lang="ts">
	import { liveQuery } from 'dexie';
	import { masterKey } from '$lib/vault';
	import {
		liveCompletedTasks,
		liveArchivedTasks,
		restoreTask,
		unarchiveCompleted,
		type Task
	} from '$lib/db';
	import { History, CheckCircle2, Archive, RotateCcw } from '@lucide/svelte';

	let completed = $state<Task[]>([]);
	let archived = $state<Task[]>([]);
	let tab = $state<'completed' | 'archived'>('completed');

	$effect(() => {
		if (!$masterKey) return;
		const c = liveCompletedTasks().subscribe((list) => (completed = list));
		const a = liveArchivedTasks().subscribe((list) => (archived = list));
		return () => {
			c.unsubscribe();
			a.unsubscribe();
		};
	});

	const list = $derived(tab === 'completed' ? completed : archived);
</script>

{#if !$masterKey}
	<p class="error">Please unlock the app from the home screen.</p>
{:else}
	<h2 class="page-title"><History size={24} /> History</h2>

	<div class="history-tabs" role="tablist" aria-label="History tabs">
		<button
			class="tab"
			class:tab-active={tab === 'completed'}
			onclick={() => (tab = 'completed')}
			role="tab"
			aria-selected={tab === 'completed'}
			aria-label="Completed"
		>
			<CheckCircle2 size={18} />
			<span>Completed</span>
		</button>
		<button
			class="tab"
			class:tab-active={tab === 'archived'}
			onclick={() => (tab = 'archived')}
			role="tab"
			aria-selected={tab === 'archived'}
			aria-label="Archived"
		>
			<Archive size={18} />
			<span>Archived</span>
		</button>
	</div>

	{#if list.length === 0}
		<div class="empty-state">
			{#if tab === 'completed'}
				<CheckCircle2 size={48} />
				<p>No completed tasks yet.</p>
			{:else}
				<Archive size={48} />
				<p>No archived tasks yet.</p>
			{/if}
		</div>
	{:else}
		<ul class="history-list">
			{#each list as task (task.id)}
				<li class="card">
					<div class="card-row">
						<div class="grow">
							<a class="task-title" href="/task/{task.id}">{task.title}</a>
							<p class="task-meta">
								{task.category}{task.dueDate ? ' · ' + new Date(task.dueDate).toLocaleDateString() : ''}
							</p>
						</div>
						<button class="outlined" onclick={() => (tab === 'archived' ? restoreTask(task.id) : unarchiveCompleted(task.id))}>
							<RotateCcw size={18} />
							<span>Restore</span>
						</button>
					</div>
				</li>
			{/each}
		</ul>
	{/if}
{/if}
