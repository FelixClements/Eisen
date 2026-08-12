<script lang="ts">
	import { liveQuery } from 'dexie';
	import { browser } from '$app/environment';
	import { db, addTodo, toggleTodo, deleteTodo, type Quadrant, type Todo } from '$lib/db';
	import { unlock } from '$lib/vault';
	import { sync } from '$lib/sync';

	let newTitle = $state('');
	let newNotes = $state('');
	let newQuadrant = $state<Quadrant>('ui');

	let password = $state('');
	let masterKey = $state<CryptoKey | null>(null);
	let error = $state<string | null>(null);
	let syncing = $state(false);

	let todos = $state<Todo[]>([]);

	$effect(() => {
		if (!browser) return;
		const query = liveQuery(() =>
			db.todos
				.where('deleted')
				.notEqual(1)
				.toArray()
				.then((list) => list.sort((a, b) => b.local_updated_at - a.local_updated_at))
		);
		const sub = query.subscribe((list) => {
			todos = list;
		});
		return () => sub.unsubscribe();
	});

	async function handleUnlock(event: Event) {
		event.preventDefault();
		try {
			masterKey = await unlock(password);
			error = null;
		} catch (e) {
			error = 'Failed to unlock. Password may be incorrect.';
		}
	}

	async function handleSubmit(event: Event) {
		event.preventDefault();
		if (!newTitle.trim()) return;
		await addTodo(newTitle.trim(), newNotes.trim(), newQuadrant);
		newTitle = '';
		newNotes = '';
	}

	async function handleSync() {
		if (!masterKey) return;
		syncing = true;
		try {
			await sync(masterKey);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Sync failed';
		} finally {
			syncing = false;
		}
	}

	const quadrantLabels: Record<Quadrant, string> = {
		ui: 'Urgent & Important',
		un: 'Urgent, Not Important',
		in: 'Important, Not Urgent',
		nn: 'Not Urgent, Not Important'
	};
</script>

<main>
	<h1>Eisen</h1>
	<p>A local-first, end-to-end encrypted PWA.</p>

	{#if !masterKey}
		<form onsubmit={handleUnlock}>
			<input type="password" placeholder="Passphrase" bind:value={password} />
			<button class="primary" type="submit">Unlock</button>
		</form>
	{:else}
		<form onsubmit={handleSubmit}>
			<input type="text" placeholder="Title" bind:value={newTitle} />
			<textarea placeholder="Notes" bind:value={newNotes}></textarea>
			<select bind:value={newQuadrant}>
				{#each Object.entries(quadrantLabels) as [value, label]}
					<option {value}>{label}</option>
				{/each}
			</select>
			<button class="primary" type="submit">Add todo</button>
		</form>

		<button onclick={handleSync} disabled={syncing}>{syncing ? 'Syncing...' : 'Sync now'}</button>

		<ul>
			{#each todos as todo (todo.id)}
				<li class:completed={todo.completed}>
					<strong>{todo.title}</strong>
					<span>{quadrantLabels[todo.quadrant]}</span>
					<p>{todo.notes}</p>
					<div class="actions">
						<button onclick={() => toggleTodo(todo.id)}>
							{todo.completed ? 'Restore' : 'Complete'}
						</button>
						<button onclick={() => deleteTodo(todo.id)}>Delete</button>
					</div>
				</li>
			{/each}
		</ul>
	{/if}

	{#if error}
		<p class="error">{error}</p>
	{/if}
</main>
