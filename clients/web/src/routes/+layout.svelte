<script lang="ts">
	import '../app.css';
	import { browser } from '$app/environment';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { get } from 'svelte/store';
	import { App, Page, Navbar, Panel, List, ListItem, Link, Toast } from 'konsta/svelte';
	import { authClient } from '$lib/auth-client';
	import { masterKey, vaultUserId, lockVault, tryAutoUnlock } from '$lib/vault';
	import { sync } from '$lib/sync';
	import { syncMessage } from '$lib/stores';
	import { drawerOpen } from '$lib/drawer';
	import { db } from '$lib/db';
	import { scheduleNextWake } from '$lib/notifications';
	import { initTheme, resolvedTheme } from '$lib/theme';
	import { useRegisterSW } from 'virtual:pwa-register/svelte';

	useRegisterSW({
		onRegisterError(error) {
			console.error('Service worker registration failed:', error);
		}
	});

	let { children, data } = $props();

	if (browser) initTheme();

	let toastOpen = $state(false);
	let toastText = $state('');
	let syncing = $state(false);

	const session = authClient.useSession();
	const user = $derived($session.data?.user ?? data.user);
	const isPublic = $derived(
		$page.url.pathname === '/sign-in' || $page.url.pathname === '/sign-up'
	);

	$effect(() => {
		if (!browser || !user?.id) return;
		tryAutoUnlock(user.id);
	});

	if (browser) {
		navigator.serviceWorker?.addEventListener('message', async (event) => {
			if (event.data?.type !== 'GET_DUE_REMINDERS') return;
			const port = event.ports[0];
			const uid = get(vaultUserId);
			if (!port || !uid) {
				port?.postMessage([]);
				return;
			}
			const now = Date.now();
			const tasks = await db.tasks.where('userId').equals(uid).toArray();
			const due = tasks
				.filter((t) => !t.deleted && !t.isCompleted && !t.isArchived && t.reminderAt && t.reminderAt <= now)
				.map((t) => ({ id: t.id, title: t.title }));
			port.postMessage(due);
		});
	}

	export async function runSync() {
		if (!$masterKey || !user?.id) return;
		syncing = true;
		syncMessage.set('');
		try {
			await sync(user.id, $masterKey);
			toastText = 'Synced';
			toastOpen = true;
			await scheduleNextWake(user.id);
		} catch (e) {
			const msg = e instanceof Error ? e.message : 'Sync failed';
			syncMessage.set(msg);
			toastText = msg;
			toastOpen = true;
		} finally {
			syncing = false;
		}
	}
</script>

<svelte:head>
	<link rel="manifest" href="/manifest.webmanifest" />
</svelte:head>

<App theme={$resolvedTheme} safeAreas materialTouchRipple={$resolvedTheme === 'material'}>
	{#if user && !isPublic}
		<Panel side="left" opened={$drawerOpen} onBackdropClick={() => drawerOpen.set(false)}>
			<Page>
				<Navbar title="Eisen">
					{#snippet right()}
						<Link iconOnly onclick={() => drawerOpen.set(false)}>✕</Link>
					{/snippet}
				</Navbar>
				<List strong inset>
					<ListItem link title="Home" href="/" onclick={() => drawerOpen.set(false)} />
					<ListItem link title="History" href="/history" onclick={() => drawerOpen.set(false)} />
					<ListItem link title="Settings" href="/settings" onclick={() => drawerOpen.set(false)} />
					<ListItem
						link
						title="Keyboard shortcuts"
						href="/keyboard-shortcuts"
						onclick={() => drawerOpen.set(false)}
					/>
				</List>
			</Page>
		</Panel>
	{/if}

	{@render children()}

	<Toast opened={toastOpen} position="center">
		<div class="px-4 py-2">{toastText}</div>
	</Toast>
</App>
