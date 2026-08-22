<script lang="ts">
	import { goto } from '$app/navigation';
	import { Page, Navbar, Block, List, ListInput, Button } from 'konsta/svelte';
	import { authenticateWithVault } from '$lib/workspace/authenticate';
	import { EisenErrorException } from '$lib/workspace/types';

	let name = $state('');
	let email = $state('');
	let password = $state('');
	let error = $state('');
	let busy = $state(false);

	async function handleSignUp(e: Event) {
		e.preventDefault();
		busy = true;
		error = '';
		try {
			await authenticateWithVault({ mode: 'sign-up', email, password, name });
			password = '';
			goto('/');
		} catch (err) {
			if (err instanceof EisenErrorException) {
				error =
					err.error.code === 'weak-passphrase' ? err.error.reason : 'Could not create your account.';
			} else {
				error = err instanceof Error ? err.message : 'Sign up failed';
			}
		} finally {
			busy = false;
		}
	}
</script>

<Page>
	<Navbar title="Create account" />
	<Block strong inset class="space-y-4">
		<p>
			Create your Eisen account. Your password signs you in and encrypts your tasks. We cannot read
			them, and a password reset cannot recover them.
		</p>
		{#if error}
			<p class="text-red-600">{error}</p>
		{/if}
		<form onsubmit={handleSignUp} class="space-y-4">
			<List strongIos outlineIos>
				<ListInput label="Name" type="text" placeholder="Your name" bind:value={name} />
				<ListInput label="Email" type="email" placeholder="you@example.com" bind:value={email} />
				<ListInput label="Password" type="password" placeholder="Account password" bind:value={password} />
			</List>
			<Button large rounded onclick={handleSignUp} disabled={busy}>
				{busy ? 'Creating…' : 'Create account'}
			</Button>
		</form>
		<Button clear onclick={() => goto('/sign-in')}>Already have an account?</Button>
	</Block>
</Page>
