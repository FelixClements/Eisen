<script lang="ts">
	import { goto } from '$app/navigation';
	import { Page, Navbar, NavbarBackLink, Block, List, ListItem } from 'konsta/svelte';
</script>

<Page>
	<Navbar title="Keyboard shortcuts">
		{#snippet left()}
			<NavbarBackLink onclick={() => goto('/')} />
		{/snippet}
	</Navbar>
	<List strong inset>
		<ListItem title="Q" after="Jump to Do Now" />
		<ListItem title="W" after="Jump to Schedule" />
		<ListItem title="E" after="Jump to Delegate" />
		<ListItem title="R" after="Jump to Eliminate" />
		<ListItem title="A" after="Add new task" />
	</List>
	<Block strong inset>
		<p class="text-sm opacity-70">Shortcut keys can be added in a future update.</p>
	</Block>
</Page>
