<script lang="ts">
	import { goto } from '$app/navigation';
	import { tick } from 'svelte';
	import { Page, Navbar, Block, List, ListInput, Button } from 'konsta/svelte';
	import { vaultSession } from '$lib/workspace/authenticate';
	import { EisenErrorException } from '$lib/workspace/types';
	import { authClient } from '$lib/auth-client';
	import { browser } from '$app/environment';

	let email = $state('');
	let password = $state('');
	let error = $state('');
	let busy = $state(false);

	$effect(() => {
		if (!browser) return;
		(async () => {
			const session = await authClient.getSession();
			const user = session.data?.user;
			if (!user) return;
			const key = await vaultSession.resume(user.id);
			if (key) goto('/');
		})();
	});

	function readForm(e: Event) {
		const form = e.currentTarget;
		if (!(form instanceof HTMLFormElement)) return;
		const fd = new FormData(form);
		const nextEmail = String(fd.get('email') ?? '').trim();
		const nextPassword = String(fd.get('password') ?? '');
		if (nextEmail) email = nextEmail;
		if (nextPassword) password = nextPassword;
	}

	async function handleSignIn(e: Event) {
		e.preventDefault();
		readForm(e);
		busy = true;
		error = '';
		await tick();
		try {
			await vaultSession.unlock({ mode: 'sign-in', email, password });
			password = '';
			goto('/');
		} catch (err) {
			if (err instanceof EisenErrorException) {
				error =
					err.error.code === 'weak-passphrase' ? err.error.reason : 'Could not open your tasks.';
			} else {
				error = err instanceof Error && err.message ? err.message : 'Sign in failed';
			}
		} finally {
			busy = false;
		}
	}
</script>

<Page>
	<Navbar title="Sign in" />
	<Block strong inset class="space-y-4">
		<p>Sign in with your email and password. The same password encrypts your tasks on this device.</p>
		{#if error}
			<p class="text-red-600">{error}</p>
		{/if}
		{#if busy}
			<p>Signing in… this can take a few seconds.</p>
		{/if}
		<form onsubmit={handleSignIn} class="space-y-4">
			<List strongIos outlineIos>
				<ListInput
					label="Email"
					name="email"
					type="email"
					placeholder="you@example.com"
					bind:value={email}
				/>
				<ListInput
					label="Password"
					name="password"
					type="password"
					placeholder="Account password"
					bind:value={password}
				/>
			</List>
			<Button large rounded type="submit" disabled={busy}>
				{busy ? 'Signing in…' : 'Sign in'}
			</Button>
		</form>
		<Button clear type="button" onclick={() => goto('/sign-up')}>Create an account</Button>
	</Block>
</Page>
