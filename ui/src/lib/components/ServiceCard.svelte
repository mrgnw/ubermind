<script lang="ts">
	import type { ServiceInfo } from '$lib/api';
	import { startService, stopService, reloadService } from '$lib/api';

	let {
		service,
		onUpdate,
	}: {
		service: ServiceInfo;
		onUpdate: () => void;
	} = $props();

	let loading = $state(false);

	async function handleAction(action: 'start' | 'stop' | 'reload') {
		loading = true;
		try {
			if (action === 'start') await startService(service.name);
			else if (action === 'stop') await stopService(service.name);
			else await reloadService(service.name);
			setTimeout(onUpdate, 500);
		} catch (e) {
			console.error(e);
		} finally {
			loading = false;
		}
	}
</script>

<div class="card" class:running={service.running}>
	<div class="header">
		<a href="/service/{service.name}" class="name">{service.name}</a>
		<span class="status" class:running={service.running}>
			{service.running ? 'running' : 'stopped'}
		</span>
	</div>

	<div class="dir">{service.dir}</div>

	<div class="actions">
		{#if service.running}
			<button onclick={() => handleAction('stop')} disabled={loading}>stop</button>
			<button onclick={() => handleAction('reload')} disabled={loading}>reload</button>
			<a href="/service/{service.name}" class="btn">echo</a>
		{:else}
			<button onclick={() => handleAction('start')} disabled={loading}>start</button>
		{/if}
	</div>
</div>

<style>
	.card {
		border: 1px solid #333;
		border-radius: 8px;
		padding: 16px;
		background: #1a1a2e;
		transition: border-color 0.2s;
	}

	.card.running {
		border-color: #2d5a2d;
	}

	.header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 8px;
	}

	.name {
		font-size: 1.1rem;
		font-weight: 600;
		color: #e0e0e0;
		text-decoration: none;
	}

	.name:hover {
		text-decoration: underline;
	}

	.status {
		font-size: 0.85rem;
		padding: 2px 8px;
		border-radius: 4px;
		background: #4a1a1a;
		color: #ff6b6b;
	}

	.status.running {
		background: #1a4a1a;
		color: #6bff6b;
	}

	.dir {
		font-size: 0.8rem;
		color: #888;
		margin-bottom: 12px;
		font-family: monospace;
	}

	.actions {
		display: flex;
		gap: 8px;
	}

	button, .btn {
		padding: 4px 12px;
		border: 1px solid #444;
		border-radius: 4px;
		background: #2a2a3e;
		color: #ccc;
		cursor: pointer;
		font-size: 0.85rem;
		text-decoration: none;
		display: inline-block;
	}

	button:hover, .btn:hover {
		background: #3a3a4e;
		border-color: #666;
	}

	button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>
