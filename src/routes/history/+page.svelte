<script lang="ts">
	import { goto } from '$app/navigation';
	import { Page, Navbar, NavbarBackLink, Block, List, ListItem, Segmented, SegmentedButton } from 'konsta/svelte';
	import { currentOpen } from '$lib/workspace/current.svelte';

	const open = $derived(currentOpen());
	let tab = $state<'completed' | 'archived'>('completed');
	const list = $derived(open ? (tab === 'completed' ? open.history.completed : open.history.archived) : []);
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

	{#if !open}
		<Block strong inset><p>Loading…</p></Block>
	{:else}
		<List strong outline>
			{#each list as task (task.id)}
				<ListItem link title={task.title} subtitle={task.notes} href="/task/{task.id}">
					{#snippet after()}
						{#if tab === 'archived'}
							<button
								type="button"
								class="text-primary text-sm"
								onclick={(e) => {
									e.preventDefault();
									open.apply({ kind: 'archive', id: task.id, archived: false });
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
