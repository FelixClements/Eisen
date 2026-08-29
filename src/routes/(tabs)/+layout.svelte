<script lang="ts">
	import AppTabbar from '$lib/components/AppTabbar.svelte';

	let { children } = $props();
</script>

<div class="app-shell">
	{@render children()}
	<AppTabbar />
</div>
