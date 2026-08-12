<script lang="ts">
	import { liveQuery } from 'dexie';
	import { masterKey } from '$lib/vault';
	import { liveCompletedTasks, liveArchivedTasks, restoreTask, unarchiveCompleted, type Task } from '$lib/db';

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
	<h2>History</h2>
	<div class="row-actions">
		<button class:primary={tab === 'completed'} onclick={() => (tab = 'completed')}>Completed</button>
		<button class:primary={tab === 'archived'} onclick={() => (tab = 'archived')}>Archived</button>
	</div>

	{#if list.length === 0}
		<div class="empty-state">No {tab} tasks.</div>
	{:else}
		<ul>
			{#each list as task (task.id)}
				<li class="card">
					<div class="card-row">
						<div class="grow">
							<a class="task-title" href="/task/{task.id}">{task.title}</a>
							<p class="task-meta">{task.category}{task.dueDate ? ' · ' + new Date(task.dueDate).toLocaleDateString() : ''}</p>
						</div>
						<button
							onclick={() =>
								tab === 'archived' ? restoreTask(task.id) : unarchiveCompleted(task.id)}
						>
							Restore
						</button>
					</div>
				</li>
			{/each}
		</ul>
	{/if}
{/if}
