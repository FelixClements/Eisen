<script lang="ts">
	import { goto } from '$app/navigation';
	import { Page, Navbar, NavbarBackLink, Block, Button, Segmented, SegmentedButton } from 'konsta/svelte';
	import { appearanceMode, setAppearanceMode } from '$lib/theme';
	import { authClient } from '$lib/auth-client';
	import { masterKey, lockVault } from '$lib/vault';
	import {
		exportRecoveryPackage,
		importRecoveryPackage,
		backupToCloud,
		listCloudBackups
	} from '$lib/recovery';
	import { subscribeToPush, notificationsSupported } from '$lib/notifications';
	import { sync } from '$lib/sync';
	import { unlockVault } from '$lib/vault';

	let { data } = $props();
	const userId = $derived(data.user?.id ?? '');

	let message = $state('');
	let importFile = $state<File | null>(null);
	let cloudBackups = $state<{ packageId: string; createdAt: number }[]>([]);
	let busy = $state(false);
	let pushEnabled = $state(false);

	async function handleExport() {
		if (!$masterKey || !userId) {
			message = 'Unlock your vault first.';
			return;
		}
		const password = prompt('Enter your vault passphrase to encrypt the recovery package:');
		if (!password) return;
		try {
			const blob = await exportRecoveryPackage(userId, password);
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = `eisen-recovery-${Date.now()}.json`;
			a.click();
			URL.revokeObjectURL(url);
			message = 'Recovery package exported.';
		} catch (e) {
			message = e instanceof Error ? e.message : 'Export failed.';
		}
	}

	async function handleImport() {
		if (!importFile || !userId) return;
		const password = prompt('Enter the vault passphrase for this package:');
		if (!password) return;
		try {
			await importRecoveryPackage(userId, importFile, password);
			await unlockVault(userId, password);
			message = 'Recovery package imported.';
			importFile = null;
		} catch (e) {
			message = e instanceof Error ? e.message : 'Import failed.';
		}
	}

	async function handleCloudBackup() {
		if (!$masterKey || !userId) return;
		const password = prompt('Enter your vault passphrase:');
		if (!password) return;
		busy = true;
		try {
			const id = await backupToCloud(userId, password);
			message = `Cloud backup created: ${id}`;
		} catch (e) {
			message = e instanceof Error ? e.message : 'Backup failed.';
		} finally {
			busy = false;
		}
	}

	async function handleSync() {
		if (!$masterKey || !userId) return;
		try {
			await sync(userId, $masterKey);
			message = 'Synced successfully.';
		} catch (e) {
			message = e instanceof Error ? e.message : 'Sync failed.';
		}
	}

	async function handlePush() {
		if (!userId) return;
		pushEnabled = await subscribeToPush(userId);
		message = pushEnabled ? 'Push notifications enabled.' : 'Could not enable push notifications.';
	}

	async function handleSignOut() {
		await lockVault();
		await authClient.signOut();
		goto('/sign-in');
	}
</script>

<Page>
	<Navbar title="Settings">
		{#snippet left()}
			<NavbarBackLink onclick={() => goto('/')} />
		{/snippet}
	</Navbar>

	<Block strong inset class="space-y-6">
		<section>
			<h3 class="mb-2 font-semibold">Appearance</h3>
			<Segmented strong rounded>
				<SegmentedButton
					active={$appearanceMode === 'system'}
					onclick={() => setAppearanceMode('system')}
				>
					System
				</SegmentedButton>
				<SegmentedButton active={$appearanceMode === 'ios'} onclick={() => setAppearanceMode('ios')}>
					iOS
				</SegmentedButton>
				<SegmentedButton
					active={$appearanceMode === 'material'}
					onclick={() => setAppearanceMode('material')}
				>
					Material
				</SegmentedButton>
			</Segmented>
			<p class="mt-2 text-sm opacity-70">
				System uses Material on Android and iOS styling on iPhone and iPad. Desktop defaults to iOS.
			</p>
		</section>

		<section>
			<h3 class="mb-2 font-semibold">Vault</h3>
			{#if $masterKey}
				<Button outline onclick={handleSync}>Sync now</Button>
				<Button outline class="mt-2" onclick={lockVault}>Lock vault</Button>
			{:else}
				<p class="text-sm opacity-70">Vault is locked.</p>
			{/if}
		</section>

		<section>
			<h3 class="mb-2 font-semibold">Backup & recovery</h3>
			<div class="flex flex-col gap-2">
				<Button outline onclick={handleExport} disabled={!$masterKey}>Export recovery package</Button>
				<input type="file" accept=".json" onchange={(e) => (importFile = (e.target as HTMLInputElement).files?.[0] ?? null)} />
				<Button outline onclick={handleImport} disabled={!importFile}>Import recovery package</Button>
				<Button outline onclick={handleCloudBackup} disabled={!$masterKey || busy}>Back up to cloud</Button>
				<Button
					clear
					onclick={async () => {
						cloudBackups = await listCloudBackups();
					}}>List cloud backups</Button
				>
				{#if cloudBackups.length}
					<ul class="text-sm">
						{#each cloudBackups as b (b.packageId)}
							<li>{b.packageId} — {new Date(b.createdAt).toLocaleString()}</li>
						{/each}
					</ul>
				{/if}
			</div>
		</section>

		<section>
			<h3 class="mb-2 font-semibold">Notifications</h3>
			{#if notificationsSupported()}
				<Button outline onclick={handlePush}>{pushEnabled ? 'Push enabled' : 'Enable push reminders'}</Button>
				<p class="mt-2 text-sm opacity-70">
					Reminders use a wake-clock: the server only stores when to nudge your device, never task content.
				</p>
			{:else}
				<p class="text-sm opacity-70">Notifications are not supported in this browser.</p>
			{/if}
		</section>

		<section>
			<h3 class="mb-2 font-semibold">Account</h3>
			<Button outline onclick={handleSignOut}>Sign out</Button>
		</section>

		{#if message}
			<p class="text-sm">{message}</p>
		{/if}
	</Block>
</Page>
