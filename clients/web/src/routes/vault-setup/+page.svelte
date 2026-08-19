<script lang="ts">
	import { goto } from '$app/navigation';
	import { Page, Navbar, Block, List, ListInput, ListItem, Button, Toggle } from 'konsta/svelte';
	import { setupVault, vaultExists } from '$lib/vault';

	let { data } = $props();
	const userId = $derived(data.user?.id ?? '');

	let passphrase = $state('');
	let confirm = $state('');
	let keepSignedIn = $state(false);
	let error = $state('');
	let busy = $state(false);
	let exists = $state(false);

	$effect(() => {
		if (!userId) return;
		vaultExists(userId).then((v) => {
			exists = v;
			if (v) goto('/');
		});
	});

	async function handleSetup(e: Event) {
		e.preventDefault();
		if (passphrase.length < 8) {
			error = 'Passphrase must be at least 8 characters.';
			return;
		}
		if (passphrase !== confirm) {
			error = 'Passphrases do not match.';
			return;
		}
		busy = true;
		error = '';
		try {
			await setupVault(userId, passphrase, keepSignedIn);
			goto('/');
		} catch (e) {
			error = e instanceof Error ? e.message : 'Vault setup failed';
		} finally {
			busy = false;
		}
	}
</script>

<Page>
	<Navbar title="Set up vault" />
	<Block strong inset class="space-y-4">
		<p>
			Choose a <strong>vault passphrase</strong> to encrypt your tasks. We cannot recover this passphrase if you
			lose it. Your account password only controls sign-in.
		</p>
		{#if error}
			<p class="text-red-600">{error}</p>
		{/if}
		<form onsubmit={handleSetup} class="space-y-4">
			<List strongIos outlineIos>
				<ListInput label="Vault passphrase" type="password" bind:value={passphrase} />
				<ListInput label="Confirm passphrase" type="password" bind:value={confirm} />
				<ListItem title="Keep vault unlocked on this device">
					{#snippet after()}
						<Toggle bind:checked={keepSignedIn} />
					{/snippet}
				</ListItem>
			</List>
			<Button large rounded onclick={handleSetup} disabled={busy}>Create vault</Button>
		</form>
	</Block>
</Page>
