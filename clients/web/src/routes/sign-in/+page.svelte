<script lang="ts">
	import { goto } from '$app/navigation';
	import { Page, Navbar, Block, List, ListInput, Button, BlockTitle } from 'konsta/svelte';
	import { authClient } from '$lib/auth-client';

	let email = $state('');
	let password = $state('');
	let name = $state('');
	let error = $state('');
	let busy = $state(false);

	async function handleSignIn(e: Event) {
		e.preventDefault();
		busy = true;
		error = '';
		try {
			const { error: err } = await authClient.signIn.email({ email, password });
			if (err) throw new Error(err.message ?? 'Sign in failed');
			goto('/');
		} catch (e) {
			error = e instanceof Error ? e.message : 'Sign in failed';
		} finally {
			busy = false;
		}
	}
</script>

<Page>
	<Navbar title="Sign in" />
	<Block strong inset class="space-y-4">
		<p>Sign in to Eisen. Your tasks are encrypted with a separate vault passphrase.</p>
		{#if error}
			<p class="text-red-600">{error}</p>
		{/if}
		<form onsubmit={handleSignIn} class="space-y-4">
			<List strongIos outlineIos>
				<ListInput label="Email" type="email" placeholder="you@example.com" bind:value={email} />
				<ListInput label="Password" type="password" placeholder="Account password" bind:value={password} />
			</List>
			<Button large rounded onclick={handleSignIn} disabled={busy}>
				{busy ? 'Signing in…' : 'Sign in'}
			</Button>
		</form>
		<Button clear onclick={() => goto('/sign-up')}>Create an account</Button>
	</Block>
</Page>
