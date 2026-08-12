<script lang="ts">
	const shortcuts = [
		{ key: 'A', action: 'Open New task composer' },
		{ key: 'Q', action: 'Jump to Do Now section' },
		{ key: 'W', action: 'Jump to Schedule section' },
		{ key: 'E', action: 'Jump to Delegate section' },
		{ key: 'R', action: 'Jump to Eliminate section' }
	];
</script>

<h2>Keyboard shortcuts</h2>

<p class="empty-state">Keyboard shortcuts are not yet active in the PWA. This list matches the Android client:</p>

<ul class="card">
	{#each shortcuts as shortcut}
		<li class="card-row">
			<strong>{shortcut.key}</strong>
			<span>{shortcut.action}</span>
		</li>
	{/each}
</ul>
