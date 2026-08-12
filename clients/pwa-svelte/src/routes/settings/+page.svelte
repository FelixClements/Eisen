<script lang="ts">
	import { masterKey, lock, unlock } from '$lib/vault';
	import { browser } from '$app/environment';
	import { exportRecoveryPackage, importRecoveryPackage } from '$lib/recovery';
	import { initiatePairing, claimPairingCode } from '$lib/pairing';
	import { backupToCloud, listCloudBackups } from '$lib/backup';

	let notifications = $state(false);
	let importFile = $state<File | null>(null);
	let message = $state('');

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

	async function handleExport() {
		if (!$masterKey) {
			message = 'Unlock the vault first.';
			return;
		}
		const password = prompt('Enter your passphrase to encrypt the recovery package:');
		if (!password) return;
		try {
			const blob = await exportRecoveryPackage(password);
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

	let pairingCode = $state('');
	let pairingExpires = $state<number | null>(null);
	let isPairing = $state(false);
	let cloudBackups = $state<{ packageId: string; createdAt: number }[]>([]);
	let isBackingUp = $state(false);

	async function handleInitiatePairing() {
		if (!$masterKey) {
			message = 'Unlock the vault first.';
			return;
		}
		try {
			isPairing = true;
			const result = await initiatePairing();
			pairingCode = result.code;
			pairingExpires = result.expiresAt;
		} catch (e) {
			message = e instanceof Error ? e.message : 'Failed to start pairing.';
		} finally {
			isPairing = false;
		}
	}

	async function handleClaimPairing() {
		const code = prompt('Enter the 6-character pairing code from your other device:');
		if (!code) return;
		try {
			isPairing = true;
			await claimPairingCode(code);
			message = 'This device is now paired. Unlock to continue.';
		} catch (e) {
			message = e instanceof Error ? e.message : 'Failed to claim pairing.';
		} finally {
			isPairing = false;
		}
	}

	async function handleCloudBackup() {
		if (!$masterKey) {
			message = 'Unlock the vault first.';
			return;
		}
		const password = prompt('Enter your passphrase to encrypt the cloud backup:');
		if (!password) return;
		try {
			isBackingUp = true;
			const packageId = await backupToCloud(password);
			message = `Cloud backup created: ${packageId}`;
		} catch (e) {
			message = e instanceof Error ? e.message : 'Cloud backup failed.';
		} finally {
			isBackingUp = false;
		}
	}

	async function handleListCloudBackups() {
		try {
			cloudBackups = await listCloudBackups();
		} catch (e) {
			message = e instanceof Error ? e.message : 'Failed to list cloud backups.';
		}
	}

	async function handleImport() {
		if (!importFile) {
			message = 'Choose a recovery package file first.';
			return;
		}
		const password = prompt('Enter the passphrase for this recovery package:');
		if (!password) return;
		try {
			await importRecoveryPackage(importFile, password);
			await unlock(password);
			message = 'Recovery package imported. Refresh if needed.';
			importFile = null;
		} catch (e) {
			message = e instanceof Error ? e.message : 'Import failed.';
		}
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
	<h3>Backup & Recovery</h3>
	<button onclick={handleExport} disabled={!$masterKey}>Export recovery package</button>
	<div class="import-row">
		<label for="import-file">Import recovery package:</label>
		<input id="import-file" type="file" accept=".json" onchange={(e) => (importFile = (e.target as HTMLInputElement).files?.[0] ?? null)} />
		<button onclick={handleImport} disabled={!importFile}>Import</button>
	</div>
</div>

<div class="card">
	<h3>Multi-Device Pairing</h3>
	<button onclick={handleInitiatePairing} disabled={!$masterKey || isPairing}>
		{isPairing ? 'Generating…' : 'Generate pairing code'}
	</button>
	{#if pairingCode}
		<p class="pairing-code">{pairingCode}</p>
		{#if pairingExpires}
			<p>Expires at {new Date(pairingExpires).toLocaleTimeString()}</p>
		{/if}
	{/if}
	<button onclick={handleClaimPairing} disabled={isPairing}>
		{isPairing ? 'Claiming…' : 'Claim pairing code'}
	</button>
</div>

<div class="card">
	<h3>Cloud Backup</h3>
	<button onclick={handleCloudBackup} disabled={!$masterKey || isBackingUp}>
		{isBackingUp ? 'Backing up…' : 'Back up to cloud'}
	</button>
	<button onclick={handleListCloudBackups}>List cloud backups</button>
	{#if cloudBackups.length > 0}
		<ul>
			{#each cloudBackups as backup (backup.packageId)}
				<li>{backup.packageId} — {new Date(backup.createdAt).toLocaleString()}</li>
			{/each}
		</ul>
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

{#if message}
	<p class="error">{message}</p>
{/if}
