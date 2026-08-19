<script lang="ts">
	import { goto } from '$app/navigation';
	import { Page, Navbar, Block, List, ListInput, Button } from 'konsta/svelte';
	import { authClient } from '$lib/auth-client';

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
			const { error: err } = await authClient.signUp.email({ email, password, name: name || email.split('@')[0] });
			if (err) throw new Error(err.message ?? 'Sign up failed');
			goto('/vault-setup');
		} catch (e) {
			error = e instanceof Error ? e.message : 'Sign up failed';
		} finally {
			busy = false;
		}
	}
</script>

<Page>
	<Navbar title="Create account" />
	<Block strong inset class="space-y-4">
		<p>Create your Eisen account. Next you will set a vault passphrase that encrypts your tasks.</p>
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
