<script lang="ts">
	import { masterKey, lock } from '$lib/vault';
	import { browser } from '$app/environment';

	let notifications = $state(false);

	$effect(() => {
		if (browser && 'Notification' in window) {
			notifications = Notification.permission === 'granted';
		}
	});

	async function requestNotifications() {
		if (!('Notification' in window)) return;
		const result = await Notification.requestPermission();
		notifications = result === 'granted';
	}
</script>

<h2>Settings</h2>

<div class="card">
	<h3>Vault</h3>
	{#if $masterKey}
		<button onclick={lock}>Lock and clear key</button>
	{:else}
		<p>Locked. Unlock from the home screen.</p>
	{/if}
</div>

<div class="card">
	<h3>Notifications</h3>
	{#if 'Notification' in window}
		<p>Permission: {notifications ? 'granted' : 'not granted'}</p>
		<button onclick={requestNotifications} disabled={notifications}>
			{notifications ? 'Already granted' : 'Request notifications'}
		</button>
	{:else}
		<p>Notifications are not supported in this browser.</p>
	{/if}
</div>

<div class="card">
	<h3>Privacy</h3>
	<p>All task data is stored locally on this device. Sync uses end-to-end encryption. No one, including the server, can read your tasks.</p>
</div>
