<script lang="ts">
	import { browser } from '$app/environment';
	import { goto } from '$app/navigation';
	import {
		Page,
		Navbar,
		Block,
		List,
		ListInput,
		Button,
		Fab,
		Link,
		Toggle,
		ListItem
	} from 'konsta/svelte';
	import { masterKey, unlockVault, vaultExists, tryAutoUnlock } from '$lib/vault';
	import {
		liveActiveTasks,
		searchTasks,
		eisenhowerCategory,
		categoryOrder,
		categoryLabels,
		toggleCompleted,
		archiveTask,
		togglePin,
		type Task,
		type EisenhowerCategory
	} from '$lib/db';
	import { showSearch } from '$lib/stores';
	import { drawerOpen } from '$lib/drawer';

	let { data } = $props();
	const userId = $derived(data.user?.id ?? '');

	let passphrase = $state('');
	let error = $state('');
	let busy = $state(false);
	let keepSignedIn = $state(false);
	let hasVault = $state(false);
	let tasks = $state<Task[]>([]);
	let searchQuery = $state('');

	$effect(() => {
		if (!browser || !userId) return;
		vaultExists(userId).then((v) => {
			hasVault = v;
			if (!v) goto('/vault-setup');
		});
		tryAutoUnlock(userId);
	});

	$effect(() => {
		if (!$masterKey || !userId) return;
		const q = searchQuery.trim();
		const source = q ? searchTasks(userId, q) : liveActiveTasks(userId);
		const sub = source.subscribe((list) => {
			tasks = list;
		});
		return () => sub.unsubscribe();
	});

	async function handleUnlock(e: Event) {
		e.preventDefault();
		busy = true;
		error = '';
		try {
			await unlockVault(userId, passphrase, keepSignedIn);
			passphrase = '';
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unlock failed';
		} finally {
			busy = false;
		}
	}

	function tasksByCategory(cat: EisenhowerCategory) {
		return tasks.filter((t) => eisenhowerCategory(t) === cat);
	}

	function formatDueDate(date: number | null): string {
		if (!date) return '';
		return new Date(date).toLocaleDateString();
	}

	function taskMeta(task: Task): string {
		const parts: string[] = [];
		const due = formatDueDate(task.dueDate);
		if (due) parts.push(due);
		if (task.category) parts.push(task.category);
		else if (task.description) parts.push(task.description);
		return parts.join(' · ');
	}
</script>

<Page>
	{#if $masterKey}
		<Navbar title="Eisen">
			{#snippet left()}
				<Link iconOnly onclick={() => drawerOpen.set(true)}>☰</Link>
			{/snippet}
			{#snippet right()}
				<Link iconOnly onclick={() => goto('/settings')}>⚙</Link>
			{/snippet}
		</Navbar>

		{#if $showSearch}
			<Block strong inset>
				<input
					type="search"
					class="home-search"
					placeholder="Search tasks…"
					bind:value={searchQuery}
				/>
			</Block>
		{/if}

		<div class="home-content">
			{#each categoryOrder as cat (cat)}
				{@const info = categoryLabels[cat]}
				{@const sectionTasks = tasksByCategory(cat)}
				<div class="section-header {info.cls}">
					<span>{info.title}</span>
					<span class="badge">{sectionTasks.length}</span>
				</div>
				{#if sectionTasks.length === 0}
					<div class="empty-category">{info.desc}</div>
				{:else}
					{#each sectionTasks as task (task.id)}
						<div class="task-card">
							<div class="task-card-row">
								<input
									type="checkbox"
									checked={task.isCompleted}
									onchange={async () => {
										await toggleCompleted(task.id);
									}}
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
										aria-label={task.isPinned ? 'Unpin' : 'Pin'}
										aria-pressed={task.isPinned}
										onclick={async () => {
											await togglePin(task.id);
										}}
									>
										<svg width="20" height="20" viewBox="0 0 24 24" fill="none" aria-hidden="true">
											{#if task.isPinned}
												<path
													d="M12 17v5M9 9l1.246 1.246M15.762 15.762 17 17M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z"
													stroke="currentColor"
													stroke-width="2"
													stroke-linecap="round"
													stroke-linejoin="round"
												/>
											{:else}
												<path
													d="M12 17v5M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z"
													stroke="currentColor"
													stroke-width="2"
													stroke-linecap="round"
													stroke-linejoin="round"
												/>
											{/if}
										</svg>
									</button>
									<button
										type="button"
										class="icon-button"
										aria-label="Archive"
										onclick={async () => {
											await archiveTask(task.id);
										}}
									>
										<svg width="20" height="20" viewBox="0 0 24 24" fill="none" aria-hidden="true">
											<path
												d="M10 11v6M14 11v6M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
												stroke="currentColor"
												stroke-width="2"
												stroke-linecap="round"
												stroke-linejoin="round"
											/>
										</svg>
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
	{:else}
		<Navbar title="Unlock vault" />
		<Block strong inset class="space-y-4">
			<p>Enter your vault passphrase to decrypt your tasks.</p>
			{#if error}
				<p class="text-red-600">{error}</p>
			{/if}
			<form onsubmit={handleUnlock} class="space-y-4">
				<List strongIos outlineIos>
					<ListInput label="Vault passphrase" type="password" bind:value={passphrase} />
					<ListItem title="Keep unlocked on this device">
						{#snippet after()}
							<Toggle component="div" bind:checked={keepSignedIn} />
						{/snippet}
					</ListItem>
				</List>
				<Button large rounded onclick={handleUnlock} disabled={busy}>
					{busy ? 'Unlocking…' : 'Unlock'}
				</Button>
			</form>
		</Block>
	{/if}
</Page>
