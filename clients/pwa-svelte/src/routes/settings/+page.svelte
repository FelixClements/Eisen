<script lang="ts">
	import { masterKey, lock, unlock } from '$lib/vault';
	import { browser } from '$app/environment';
	import { exportRecoveryPackage, importRecoveryPackage } from '$lib/recovery';
	import { initiatePairing, claimPairingCode } from '$lib/pairing';
	import { backupToCloud, listCloudBackups } from '$lib/backup';
	import {
		Lock,
		Download,
		Upload,
		Smartphone,
		Cloud,
		CloudUpload,
		Bell,
		Shield,
		Loader,
		FileJson
	} from '@lucide/svelte';

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
			message = 'Paired. To unlock this device, import the recovery package from the original device (Backup & Recovery → Export), then unlock with the same passphrase.';
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

	function importLabel() {
		if (importFile) return importFile.name;
		return 'Choose recovery package…';
	}
</script>

<h2 class="page-title">Settings</h2>

<section class="settings-section">
	<h3 class="section-title"><Lock size={20} /> Vault</h3>
	{#if $masterKey}
		<button class="outlined" onclick={lock}>
			<Lock size={18} />
			<span>Lock and clear key</span>
		</button>
	{:else}
		<p class="setting-description">Locked. Unlock from the home screen.</p>
	{/if}
</section>

<section class="settings-section">
	<h3 class="section-title"><Download size={20} /> Backup & Recovery</h3>
	<p class="setting-description">Export or import an encrypted recovery package.</p>
	<div class="setting-actions">
		<button class="outlined" onclick={handleExport} disabled={!$masterKey}>
			<Download size={18} />
			<span>Export recovery package</span>
		</button>
		<div class="file-row">
			<label class="file-picker" for="import-file">
				<FileJson size={18} />
				<span>{importLabel()}</span>
			</label>
			<input id="import-file" type="file" accept=".json" class="sr-only" onchange={(e) => (importFile = (e.target as HTMLInputElement).files?.[0] ?? null)} />
			<button class="outlined" onclick={handleImport} disabled={!importFile}>
				<Upload size={18} />
				<span>Import</span>
			</button>
		</div>
	</div>
</section>

<section class="settings-section">
	<h3 class="section-title"><Smartphone size={20} /> Multi-Device Pairing</h3>
	<p class="setting-description">Add this device to cloud sync, or join from a new device.</p>
	<div class="setting-actions">
		<button class="outlined" onclick={handleInitiatePairing} disabled={!$masterKey || isPairing}>
			{#if isPairing}
				<Loader size={18} />
			{:else}
				<Smartphone size={18} />
			{/if}
			<span>{isPairing ? 'Enrolling…' : 'Add this device'}</span>
		</button>
		{#if pairingCode}
			<p class="pairing-code" aria-live="polite">{pairingCode}</p>
			{#if pairingExpires}
				<p class="setting-description">Expires at {new Date(pairingExpires).toLocaleTimeString()}</p>
			{/if}
		{/if}
		<button class="outlined" onclick={handleClaimPairing} disabled={isPairing}>
			{#if isPairing}
				<Loader size={18} />
			{:else}
				<Smartphone size={18} />
			{/if}
			<span>{isPairing ? 'Joining…' : 'Join from another device'}</span>
		</button>
	</div>
</section>

<section class="settings-section">
	<h3 class="section-title"><Cloud size={20} /> Cloud Backup</h3>
	<p class="setting-description">Back up encrypted packages to the cloud and list existing backups.</p>
	<div class="setting-actions">
		<button class="outlined" onclick={handleCloudBackup} disabled={!$masterKey || isBackingUp}>
			{#if isBackingUp}
				<Loader size={18} />
			{:else}
				<CloudUpload size={18} />
			{/if}
			<span>{isBackingUp ? 'Backing up…' : 'Back up to cloud'}</span>
		</button>
		<button class="outlined" onclick={handleListCloudBackups}>
			<Cloud size={18} />
			<span>List cloud backups</span>
		</button>
		{#if cloudBackups.length > 0}
			<ul class="backup-list">
				{#each cloudBackups as backup (backup.packageId)}
					<li>{backup.packageId} — {new Date(backup.createdAt).toLocaleString()}</li>
				{/each}
			</ul>
		{/if}
	</div>
</section>

<section class="settings-section">
	<h3 class="section-title"><Bell size={20} /> Notifications</h3>
	{#if 'Notification' in window}
		<p class="setting-description">Permission: {notifications ? 'granted' : 'not granted'}</p>
		<button class="outlined" onclick={requestNotifications} disabled={notifications}>
			<Bell size={18} />
			<span>{notifications ? 'Already granted' : 'Request notifications'}</span>
		</button>
	{:else}
		<p class="setting-description">Notifications are not supported in this browser.</p>
	{/if}
</section>

<section class="settings-section">
	<h3 class="section-title"><Shield size={20} /> Privacy</h3>
	<p class="setting-description">
		All task data is stored locally on this device. Sync uses end-to-end encryption. No one, including the server, can read your tasks.
	</p>
</section>

{#if message}
	<p class="message" class:error={message.toLowerCase().includes('failed') || message.toLowerCase().includes('unlock the vault')}>
		{message}
	</p>
{/if}
