<script lang="ts">
	import { goto } from '$app/navigation';
	import { Page, Navbar, NavbarBackLink, Block, List, ListItem, Segmented, SegmentedButton } from 'konsta/svelte';
	import { masterKey } from '$lib/vault';
	import { liveCompletedTasks, liveArchivedTasks, restoreTask, type Task } from '$lib/db';

	let { data } = $props();
	const userId = $derived(data.user?.id ?? '');

	let tab = $state<'completed' | 'archived'>('completed');
	let completed = $state<Task[]>([]);
	let archived = $state<Task[]>([]);

	$effect(() => {
		if (!$masterKey || !userId) return;
		const c = liveCompletedTasks(userId).subscribe((l) => (completed = l));
		const a = liveArchivedTasks(userId).subscribe((l) => (archived = l));
		return () => {
			c.unsubscribe();
			a.unsubscribe();
		};
	});

	const list = $derived(tab === 'completed' ? completed : archived);
</script>

<Page>
	<Navbar title="History">
		{#snippet left()}
			<NavbarBackLink onclick={() => goto('/')} />
		{/snippet}
		{#snippet subnavbar()}
			<Segmented strong rounded>
				<SegmentedButton active={tab === 'completed'} onclick={() => (tab = 'completed')}>
					Completed
				</SegmentedButton>
				<SegmentedButton active={tab === 'archived'} onclick={() => (tab = 'archived')}>
					Archived
				</SegmentedButton>
			</Segmented>
		{/snippet}
	</Navbar>

	{#if !$masterKey}
		<Block strong inset><p>Unlock your vault to view history.</p></Block>
	{:else}
		<List strong outline>
			{#each list as task (task.id)}
				<ListItem link title={task.title} subtitle={task.description} href="/task/{task.id}">
					{#snippet after()}
						{#if tab === 'archived'}
							<button
								type="button"
								class="text-primary text-sm"
								onclick={async (e) => {
									e.preventDefault();
									await restoreTask(task.id);
								}}>Restore</button
							>
						{/if}
					{/snippet}
				</ListItem>
			{:else}
				<ListItem title="Nothing here yet" />
			{/each}
		</List>
	{/if}
</Page>
