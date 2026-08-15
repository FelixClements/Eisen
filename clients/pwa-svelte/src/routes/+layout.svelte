<script lang="ts">
	import '../app.css';
	import { masterKey, lock } from '$lib/vault';
	import { sync } from '$lib/sync';
	import { syncMessage } from '$lib/stores';
	import { Menu, RefreshCw, Loader, Lock } from '@lucide/svelte';

	let { children } = $props();
	let drawerOpen = $state(false);
	let syncing = $state(false);

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
			<Menu size={24} />
		</button>
		<div class="header-actions">
			<button class="icon-button" onclick={handleSync} disabled={syncing} aria-label="Sync now">
				{#if syncing}
					<Loader size={24} />
				{:else}
					<RefreshCw size={24} />
				{/if}
			</button>
			<button class="icon-button" onclick={() => ($masterKey ? lock() : null)} aria-label="Lock">
				<Lock size={24} />
			</button>
		</div>
	{/if}
</header>

{#if drawerOpen}
	<div
		class="drawer"
		role="button"
		tabindex="-1"
		aria-label="Close menu"
		onclick={(e) => {
			if (e.target === e.currentTarget) closeDrawer();
		}}
		onkeydown={(e) => {
			if (e.key === 'Enter' || e.key === ' ') {
				e.preventDefault();
				closeDrawer();
			}
		}}
	>
		<nav class="drawer-panel" aria-label="Main navigation">
			<a class="drawer-item" href="/" onclick={closeDrawer}>Home</a>
			<a class="drawer-item" href="/history" onclick={closeDrawer}>History</a>
			<a class="drawer-item" href="/settings" onclick={closeDrawer}>Settings</a>
			<a class="drawer-item" href="/keyboard-shortcuts" onclick={closeDrawer}>Keyboard shortcuts</a>
		</nav>
	</div>
{/if}

<main class="content">
	{@render children()}
</main>
