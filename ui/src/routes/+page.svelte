<script lang="ts">
	import { onMount } from 'svelte';
	import { getServices, type ServiceInfo } from '$lib/api';
	import ServiceCard from '$lib/components/ServiceCard.svelte';

	let services = $state<ServiceInfo[]>([]);
	let error = $state('');
	let refreshTimer: ReturnType<typeof setInterval>;

	async function refresh() {
		try {
			services = await getServices();
			error = '';
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	onMount(() => {
		refresh();
		refreshTimer = setInterval(refresh, 5000);
		return () => clearInterval(refreshTimer);
	});
</script>

<div class="dashboard">
	<header>
		<h1>ubermind</h1>
		<button onclick={refresh}>refresh</button>
	</header>

	{#if error}
		<div class="error">
			{error}
			<p>Make sure the ubermind-ui server is running on port 13369</p>
		</div>
	{/if}

	<div class="grid">
		{#each services as service (service.name)}
			<ServiceCard {service} onUpdate={refresh} />
		{/each}
	</div>

	{#if services.length === 0 && !error}
		<p class="empty">No services configured. Run <code>ubermind init</code> to get started.</p>
	{/if}
</div>

<style>
	.dashboard {
		max-width: 960px;
		margin: 0 auto;
		padding: 24px;
	}

	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 24px;
	}

	h1 {
		font-size: 1.5rem;
		font-weight: 700;
		color: #e0e0e0;
		margin: 0;
	}

	header button {
		padding: 4px 12px;
		border: 1px solid #444;
		border-radius: 4px;
		background: #2a2a3e;
		color: #ccc;
		cursor: pointer;
		font-size: 0.85rem;
	}

	header button:hover {
		background: #3a3a4e;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
		gap: 16px;
	}

	.error {
		background: #4a1a1a;
		border: 1px solid #ff6b6b;
		border-radius: 8px;
		padding: 16px;
		margin-bottom: 16px;
		color: #ff6b6b;
	}

	.error p {
		margin: 8px 0 0;
		font-size: 0.85rem;
		color: #ccc;
	}

	.empty {
		color: #888;
		text-align: center;
		margin-top: 48px;
	}

	code {
		background: #2a2a3e;
		padding: 2px 6px;
		border-radius: 3px;
		font-size: 0.9rem;
	}
</style>
