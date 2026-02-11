<script lang="ts">
	let { children } = $props();
</script>

<svelte:head>
	<title>ubermind</title>
</svelte:head>

{@render children()}

<style>
	:global(body) {
		margin: 0;
		padding: 0;
		background: #0f0f1a;
		color: #e0e0e0;
		font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
	}

	:global(*) {
		box-sizing: border-box;
	}

	:global(a) {
		color: inherit;
	}
</style>
