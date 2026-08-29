<script lang="ts">
	import { page } from '$app/stores';
	import { Tabbar, TabbarLink, ToolbarPane, Icon } from 'konsta/svelte';
	import { LayoutGrid, History, Plus, Settings } from '@lucide/svelte';

	const pathname = $derived($page.url.pathname);

	const tabs = [
		{ href: '/', label: 'Matrix', icon: LayoutGrid, active: (p: string) => p === '/' },
		{ href: '/history', label: 'History', icon: History, active: (p: string) => p === '/history' },
		{ href: '/new-task', label: 'Add', icon: Plus, active: () => false },
		{ href: '/settings', label: 'Settings', icon: Settings, active: (p: string) => p === '/settings' }
	] as const;
</script>

<Tabbar
	labels
	icons
	class="app-tabbar fixed bottom-0 left-1/2 z-30 w-full max-w-lg -translate-x-1/2"
>
	<ToolbarPane>
		{#each tabs as tab (tab.href)}
			<TabbarLink
				href={tab.href}
				active={tab.active(pathname)}
				label={tab.label}
			>
				{#snippet icon()}
					<Icon>
						{#snippet ios()}
							<tab.icon class="h-7 w-7" strokeWidth={2} aria-hidden="true" />
						{/snippet}
						{#snippet material()}
							<tab.icon class="h-6 w-6" strokeWidth={2} aria-hidden="true" />
						{/snippet}
					</Icon>
				{/snippet}
			</TabbarLink>
		{/each}
	</ToolbarPane>
</Tabbar>
