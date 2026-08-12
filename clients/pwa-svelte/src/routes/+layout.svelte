<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import { masterKey, lock } from '$lib/vault';
	import { sync } from '$lib/sync';
	import { search, syncMessage } from '$lib/stores';

	let { children } = $props();
	let drawerOpen = $state(false);
	let syncing = $state(false);

	let isHome = $derived($page.url.pathname === '/');

	function closeDrawer() {
		drawerOpen = false;
	}

	async function handleSync() {
		if (!$masterKey) return;
		syncing = true;
		syncMessage.set('');
		try {
			await sync($masterKey);
		} catch (e) {
			syncMessage.set(e instanceof Error ? e.message : 'Sync failed');
		} finally {
			syncing = false;
		}
	}
</script>

<svelte:head>
	<link rel="manifest" href="/manifest.webmanifest" />
	<meta name="theme-color" content="#0f766e" />
</svelte:head>

<header class="app-header">
	{#if $masterKey}
		<button class="icon-button" onclick={() => (drawerOpen = !drawerOpen)} aria-label="Open menu">
			☰
		</button>
	{/if}
	<h1>Eisen</h1>
	{#if isHome && $masterKey}
		<input
			type="text"
			class="header-search"
			value={$search}
			oninput={(e) => search.set(e.currentTarget.value)}
			placeholder="Search tasks…"
		/>
	{/if}
	{#if $masterKey}
		<div class="header-actions">
			<button class="icon-button" onclick={handleSync} disabled={syncing} aria-label="Sync">
				{syncing ? '⟳' : '🔄'}
			</button>
			<button class="icon-button" onclick={() => ($masterKey ? lock() : null)} aria-label="Lock">🔒</button>
		</div>
	{/if}
</header>

{#if drawerOpen}
	<nav
		class="drawer"
		aria-label="Main navigation"
		onclick={(e) => {
			if (e.target === e.currentTarget) closeDrawer();
		}}
	>
		<div class="drawer-panel">
			<a class="drawer-item" href="/" onclick={closeDrawer}>Home</a>
			<a class="drawer-item" href="/history" onclick={closeDrawer}>History</a>
			<a class="drawer-item" href="/settings" onclick={closeDrawer}>Settings</a>
			<a class="drawer-item" href="/keyboard-shortcuts" onclick={closeDrawer}>Keyboard shortcuts</a>
		</div>
	</nav>
{/if}

<main class="content">
	{@render children()}
</main>
