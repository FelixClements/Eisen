<script lang="ts">
	import { goto } from '$app/navigation';
	import { Page, Navbar, Block, Fab, Link } from 'konsta/svelte';
	import { currentOpen } from '$lib/workspace/current.svelte';
	import { QUADRANT_META } from '$lib/workspace';
	import { drawerOpen } from '$lib/drawer';

	const open = $derived(currentOpen());

	function taskMeta(task: { dueAt: number | null; tag: string; notes: string }): string {
		const parts: string[] = [];
		if (task.dueAt) parts.push(new Date(task.dueAt).toLocaleDateString());
		if (task.tag) parts.push(task.tag);
		else if (task.notes) parts.push(task.notes);
		return parts.join(' · ');
	}
</script>

<Page>
	<Navbar title="Eisen">
		{#snippet left()}
			<Link iconOnly onclick={() => drawerOpen.set(true)}>☰</Link>
		{/snippet}
		{#snippet right()}
			<Link iconOnly onclick={() => goto('/settings')}>⚙</Link>
		{/snippet}
	</Navbar>

	{#if open}
		<Block strong inset>
			<input
				type="search"
				class="home-search"
				placeholder="Search tasks…"
				value={open.filter}
				oninput={(e) => {
					open.filter = (e.currentTarget as HTMLInputElement).value;
				}}
			/>
		</Block>

		<div class="home-content">
			{#each open.matrix.order as q (q)}
				{@const info = QUADRANT_META[q]}
				{@const sectionTasks = open.matrix.cells[q].tasks}
				<div class="section-header {info.cls}">
					<span>{info.label}</span>
					<span class="badge">{sectionTasks.length}</span>
				</div>
				{#if sectionTasks.length === 0}
					<div class="empty-category">{info.hint}</div>
				{:else}
					{#each sectionTasks as task (task.id)}
						<div class="task-card">
							<div class="task-card-row">
								<input
									type="checkbox"
									checked={task.completed}
									onchange={() => open.apply({ kind: 'complete', id: task.id, done: true })}
									aria-label="Mark {task.title} complete"
								/>
								<div class="task-card-body">
									<a class="task-title" href="/task/{task.id}">{task.title}</a>
									{#if taskMeta(task)}
										<p class="task-meta">{taskMeta(task)}</p>
									{/if}
								</div>
								<div class="task-actions">
									<button
										type="button"
										class="icon-button"
										aria-label={task.pinned ? 'Unpin' : 'Pin'}
										aria-pressed={task.pinned}
										onclick={() => open.apply({ kind: 'update', id: task.id, patch: { pinned: !task.pinned } })}
									>
										Pin
									</button>
									<button
										type="button"
										class="icon-button"
										aria-label="Archive"
										onclick={() => open.apply({ kind: 'archive', id: task.id, archived: true })}
									>
										Archive
									</button>
								</div>
							</div>
						</div>
					{/each}
				{/if}
			{/each}
		</div>

		<Fab class="fixed right-safe-4 bottom-safe-4 z-20" onclick={() => goto('/new-task')}>
			{#snippet icon()}
				<span class="text-2xl">+</span>
			{/snippet}
		</Fab>
	{/if}
</Page>
