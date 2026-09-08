<script lang="ts">
	import { goto } from '$app/navigation';
	import { tick } from 'svelte';
	import { Page, Navbar, Block, List, ListInput, Button } from 'konsta/svelte';
	import { vaultSession } from '$lib/workspace/authenticate';
	import { EisenErrorException } from '$lib/workspace/types';

	let name = $state('');
	let email = $state('');
	let password = $state('');
	let error = $state('');
	let busy = $state(false);

	function readForm(e: Event) {
		const form = e.currentTarget;
		if (!(form instanceof HTMLFormElement)) return;
		const fd = new FormData(form);
		const nextName = String(fd.get('name') ?? '').trim();
		const nextEmail = String(fd.get('email') ?? '').trim();
		const nextPassword = String(fd.get('password') ?? '');
		if (nextName) name = nextName;
		if (nextEmail) email = nextEmail;
		if (nextPassword) password = nextPassword;
	}

	async function handleSignUp(e: Event) {
		e.preventDefault();
		readForm(e);
		busy = true;
		error = '';
		await tick();
		try {
			await vaultSession.unlock({ mode: 'sign-up', email, password, name });
			password = '';
			goto('/');
		} catch (err) {
			if (err instanceof EisenErrorException) {
				error =
					err.error.code === 'weak-passphrase' ? err.error.reason : 'Could not create your account.';
			} else {
				error = err instanceof Error && err.message ? err.message : 'Sign up failed';
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
		{#if busy}
			<p>Creating your account… this can take a few seconds.</p>
		{/if}
		<form onsubmit={handleSignUp} class="space-y-4">
			<List strongIos outlineIos>
				<ListInput label="Name" name="name" type="text" placeholder="Your name" bind:value={name} />
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
				{busy ? 'Creating…' : 'Create account'}
			</Button>
		</form>
		<Button clear type="button" onclick={() => goto('/sign-in')}>Already have an account?</Button>
	</Block>
</Page>
