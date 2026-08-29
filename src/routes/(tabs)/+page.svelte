<script lang="ts">
	import { Pin, PinOff, Archive, Search } from '@lucide/svelte';
	import { tick } from 'svelte';
	import { Page, Navbar, Block, Link } from 'konsta/svelte';
	import { currentOpen } from '$lib/workspace/current.svelte';
	import { QUADRANT_META } from '$lib/workspace';

	const open = $derived(currentOpen());
	let searchOpen = $state(false);
	let searchInput = $state<HTMLInputElement | null>(null);

	async function toggleSearch() {
		searchOpen = !searchOpen;
		if (searchOpen) {
			await tick();
			searchInput?.focus();
		}
	}

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
		{#snippet right()}
			<Link
				iconOnly
				class={open?.filter ? 'text-primary' : undefined}
				aria-label={searchOpen ? 'Close search' : 'Search tasks'}
				aria-pressed={searchOpen}
				onclick={toggleSearch}
			>
				<Search size={22} strokeWidth={2} aria-hidden="true" />
			</Link>
		{/snippet}
	</Navbar>

	{#if open}
		{#if searchOpen}
			<Block strong inset>
				<input
					bind:this={searchInput}
					type="search"
					class="home-search"
					placeholder="Search tasks…"
					value={open.filter}
					oninput={(e) => {
						open.filter = (e.currentTarget as HTMLInputElement).value;
					}}
				/>
			</Block>
		{/if}

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
										class:icon-button-active={task.pinned}
										aria-label={task.pinned ? 'Unpin' : 'Pin'}
										aria-pressed={task.pinned}
										onclick={() => open.apply({ kind: 'update', id: task.id, patch: { pinned: !task.pinned } })}
									>
										{#if task.pinned}
											<PinOff size={18} strokeWidth={2} aria-hidden="true" />
										{:else}
											<Pin size={18} strokeWidth={2} aria-hidden="true" />
										{/if}
									</button>
									<button
										type="button"
										class="icon-button"
										aria-label="Archive"
										onclick={() => open.apply({ kind: 'archive', id: task.id, archived: true })}
									>
										<Archive size={18} strokeWidth={2} aria-hidden="true" />
									</button>
								</div>
							</div>
						</div>
					{/each}
				{/if}
			{/each}
		</div>
	{/if}
</Page>
