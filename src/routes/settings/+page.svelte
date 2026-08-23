<script lang="ts">
	import { goto } from '$app/navigation';
	import { Page, Navbar, NavbarBackLink, Block, Button, Segmented, SegmentedButton } from 'konsta/svelte';
	import { appearanceMode, setAppearanceMode } from '$lib/theme';
	import { currentOpen } from '$lib/workspace/current.svelte';

	const open = $derived(currentOpen());

	let message = $state('');
	let importFile = $state<File | null>(null);
	let cloudBackups = $state<{ id: string; createdAt: number }[]>([]);
	let busy = $state(false);

	async function handleExport() {
		if (!open) return;
		const password = prompt('Enter your account password to encrypt the recovery package:');
		if (!password) return;
		const result = await open.recovery.export(password);
		if (!result.ok) {
			message = result.error.code;
			return;
		}
		const url = URL.createObjectURL(result.value);
		const a = document.createElement('a');
		a.href = url;
		a.download = `eisen-recovery-${Date.now()}.json`;
		a.click();
		URL.revokeObjectURL(url);
		message = 'Recovery package exported.';
	}

	async function handleImport() {
		if (!importFile || !open) return;
		const password = prompt('Enter the password used when this package was exported:');
		if (!password) return;
		const result = await open.recovery.import(importFile, password);
		message = result.ok ? 'Recovery package imported.' : result.error.code;
		if (result.ok) importFile = null;
	}

	async function handleCloudBackup() {
		if (!open) return;
		const password = prompt('Enter your account password:');
		if (!password) return;
		busy = true;
		try {
			const result = await open.recovery.backup(password);
			message = result.ok ? `Cloud backup created: ${result.value.id}` : result.error.code;
		} finally {
			busy = false;
		}
	}

	async function handleSignOut() {
		if (open) await open.signOut();
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
			<h3 class="mb-2 font-semibold">Sync</h3>
			{#if open}
				<Button
					outline
					onclick={async () => {
						await open.sync.now();
						message = open.sync.lastError ? open.sync.lastError.code : 'Synced.';
					}}>Sync now</Button
				>
			{/if}
		</section>

		<section>
			<h3 class="mb-2 font-semibold">Backup & recovery</h3>
			<p class="mb-2 text-sm opacity-70">
				Optional. A second device only needs your account password. Use this if you forget that password
				and still have a file you exported earlier.
			</p>
			<div class="flex flex-col gap-2">
				<Button outline onclick={handleExport} disabled={!open}>Export recovery package</Button>
				<input
					type="file"
					accept=".json"
					onchange={(e) => (importFile = (e.currentTarget as HTMLInputElement).files?.[0] ?? null)}
				/>
				<Button outline onclick={handleImport} disabled={!importFile}>Import recovery package</Button>
				<Button outline onclick={handleCloudBackup} disabled={!open || busy}>Back up to cloud</Button>
				<Button
					clear
					onclick={async () => {
						if (!open) return;
						const result = await open.recovery.list();
						if (result.ok) cloudBackups = result.value;
						else message = result.error.code;
					}}>List cloud backups</Button
				>
				{#if cloudBackups.length}
					<ul class="text-sm">
						{#each cloudBackups as b (b.id)}
							<li>
								<button
									type="button"
									class="text-primary"
									onclick={async () => {
										if (!open) return;
										const password = prompt('Password used when exporting this backup:');
										if (!password) return;
										const result = await open.recovery.restoreCloud(b.id, password);
										message = result.ok ? 'Cloud backup restored.' : result.error.code;
									}}
								>
									{b.id} — {new Date(b.createdAt).toLocaleString()}
								</button>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		</section>

		<section>
			<h3 class="mb-2 font-semibold">Notifications</h3>
			<Button
				outline
				onclick={async () => {
					if (!open) return;
					await open.reminders.enable();
					message = 'Push reminders requested.';
				}}>Enable push reminders</Button
			>
			<p class="mt-2 text-sm opacity-70">
				Reminders use a wake-clock: the server only stores when to nudge your device, never task content.
			</p>
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
