<script lang="ts">
	import '../app.css';
	import { masterKey, lock } from '$lib/vault';

	let { children } = $props();
	let drawerOpen = $state(false);

	function closeDrawer() {
		drawerOpen = false;
	}
</script>

<svelte:head>
	<link rel="manifest" href="/manifest.webmanifest" />
	<meta name="theme-color" content="#0f766e" />
</svelte:head>

<header class="app-header">
	<button class="icon-button" onclick={() => (drawerOpen = !drawerOpen)} aria-label="Open menu">
		☰
	</button>
	<h1>Eisen</h1>
	{#if $masterKey}
		<button class="icon-button" onclick={() => ($masterKey ? lock() : null)} aria-label="Lock">🔒</button>
	{/if}
</header>

{#if drawerOpen}
	<nav class="drawer" aria-label="Main navigation" onclick={closeDrawer}>
		<div class="drawer-panel" onclick={(e) => e.stopPropagation()}>
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
