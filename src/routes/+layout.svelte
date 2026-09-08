<script lang="ts">
	import '../app.css';
	import { browser } from '$app/environment';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { untrack } from 'svelte';
	import { App, Page, Navbar } from 'konsta/svelte';
	import { authClient } from '$lib/auth-client';
	import { initTheme, resolvedTheme } from '$lib/theme';
	import { vaultSession } from '$lib/workspace/authenticate';
	import { browserReminders } from '$lib/workspace/browser-reminders';
	import { bindWorkspace, currentOpen } from '$lib/workspace/current.svelte';
	import { useRegisterSW } from 'virtual:pwa-register/svelte';

	useRegisterSW({
		onRegisterError(error: Error) {
			console.error('Service worker registration failed:', error);
		}
	});

	let { children, data } = $props();

	if (browser) initTheme();

	let bootFailed = $state(false);

	const session = authClient.useSession();
	const user = $derived($session.data?.user ?? data.user);
	const isPublic = $derived(
		$page.url.pathname === '/sign-in' || $page.url.pathname === '/sign-up'
	);
	const open = $derived(currentOpen());

	$effect(() => {
		if (!browser) return;
		if (open?.sync.lastError?.code === 'session-expired') {
			goto('/sign-in');
		}
	});

	$effect(() => {
		if (!browser) return;
		if (!user?.id) {
			untrack(() => bindWorkspace(null));
			return;
		}
		if (isPublic) return;
		let cancelled = false;
		(async () => {
			const key = await vaultSession.resume(user.id);
			if (cancelled) return;
			if (!key) {
				bootFailed = true;
				await goto('/sign-in');
				return;
			}
			bootFailed = false;
			const ws = vaultSession.openWorkspace(user.id, key, {
				reminders: browserReminders(data.vapidPublicKey),
				onSignOut: async () => {
					await vaultSession.lock(user.id);
					await authClient.signOut();
				}
			});
			untrack(() => bindWorkspace(ws));
			await ws.ready;
		})();
		return () => {
			cancelled = true;
		};
	});

	if (browser) {
		navigator.serviceWorker?.addEventListener('message', async (event) => {
			if (event.data?.type !== 'GET_DUE_REMINDERS') return;
			const port = event.ports[0];
			const state = currentOpen();
			if (!port) return;
			port.postMessage(state ? state.dueReminders() : []);
		});
	}
</script>

<svelte:head>
	<link rel="manifest" href="/manifest.webmanifest" />
</svelte:head>

<App theme={$resolvedTheme} safeAreas materialTouchRipple={$resolvedTheme === 'material'}>
	{#if isPublic}
		{@render children()}
	{:else if !user}
		<Page><Navbar title="Eisen" /></Page>
	{:else if bootFailed}
		<Page><Navbar title="Eisen" /></Page>
	{:else if !open}
		<Page>
			<Navbar title="Eisen" />
			<p class="p-4">Loading…</p>
		</Page>
	{:else}
		{@render children()}
	{/if}
</App>
