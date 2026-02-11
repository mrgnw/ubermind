<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import {
		getServiceDetail,
		getPanes,
		startService,
		stopService,
		reloadService,
		echoWebSocketUrl,
		type ServiceDetail,
		type TmuxPane,
	} from '$lib/api';
	import Terminal from '$lib/components/Terminal.svelte';

	let detail = $state<ServiceDetail | null>(null);
	let panes = $state<TmuxPane[]>([]);
	let error = $state('');
	let loading = $state(false);
	let wsUrl = $state('');

	let name = $derived(page.params.name!);

	async function refresh() {
		try {
			detail = await getServiceDetail(name);
			if (detail.running) {
				panes = await getPanes(name);
				wsUrl = echoWebSocketUrl(name);
			} else {
				panes = [];
				wsUrl = '';
			}
			error = '';
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	async function handleAction(action: 'start' | 'stop' | 'reload') {
		loading = true;
		try {
			if (action === 'start') await startService(name);
			else if (action === 'stop') await stopService(name);
			else await reloadService(name);
			setTimeout(refresh, 1000);
		} catch (e) {
			console.error(e);
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		refresh();
		const timer = setInterval(refresh, 10000);
		return () => clearInterval(timer);
	});
</script>

<div class="service-page">
	<header>
		<a href="/" class="back">← services</a>
		<h1>{name}</h1>
		{#if detail}
			<span class="status" class:running={detail.running}>
				{detail.running ? 'running' : 'stopped'}
			</span>
		{/if}
	</header>

	{#if error}
		<div class="error">{error}</div>
	{/if}

	{#if detail}
		<div class="info">
			<span class="dir">{detail.dir}</span>
			<div class="actions">
				{#if detail.running}
					<button onclick={() => handleAction('stop')} disabled={loading}>stop</button>
					<button onclick={() => handleAction('reload')} disabled={loading}>reload</button>
				{:else}
					<button onclick={() => handleAction('start')} disabled={loading}>start</button>
				{/if}
			</div>
		</div>

		{#if detail.processes.length > 0}
			<div class="processes">
				<h2>processes</h2>
				<table>
					<thead>
						<tr>
							<th>name</th>
							<th>pid</th>
							<th>status</th>
						</tr>
					</thead>
					<tbody>
						{#each detail.processes as proc}
							<tr>
								<td>{proc.name}</td>
								<td class="mono">{proc.pid ?? '-'}</td>
								<td>{proc.status}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}

		{#if panes.length > 0}
			<div class="panes-info">
				<h2>tmux panes</h2>
				<div class="pane-list">
					{#each panes as pane}
						<span class="pane-tag">
							{pane.session}:{pane.window}.{pane.pane} ({pane.command})
						</span>
					{/each}
				</div>
			</div>
		{/if}

		{#if detail.running && wsUrl}
			<div class="terminal-section">
				<h2>echo</h2>
				<Terminal {wsUrl} />
			</div>
		{/if}
	{/if}
</div>

<style>
	.service-page {
		max-width: 1100px;
		margin: 0 auto;
		padding: 24px;
	}

	header {
		display: flex;
		align-items: center;
		gap: 16px;
		margin-bottom: 20px;
	}

	.back {
		color: #888;
		text-decoration: none;
		font-size: 0.9rem;
	}

	.back:hover {
		color: #ccc;
	}

	h1 {
		font-size: 1.5rem;
		font-weight: 700;
		color: #e0e0e0;
		margin: 0;
	}

	h2 {
		font-size: 1rem;
		font-weight: 600;
		color: #aaa;
		margin: 0 0 12px;
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

	.info {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 24px;
	}

	.dir {
		font-size: 0.85rem;
		color: #888;
		font-family: monospace;
	}

	.actions {
		display: flex;
		gap: 8px;
	}

	button {
		padding: 4px 12px;
		border: 1px solid #444;
		border-radius: 4px;
		background: #2a2a3e;
		color: #ccc;
		cursor: pointer;
		font-size: 0.85rem;
	}

	button:hover {
		background: #3a3a4e;
		border-color: #666;
	}

	button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.error {
		background: #4a1a1a;
		border: 1px solid #ff6b6b;
		border-radius: 8px;
		padding: 16px;
		margin-bottom: 16px;
		color: #ff6b6b;
	}

	.processes {
		margin-bottom: 24px;
	}

	table {
		width: 100%;
		border-collapse: collapse;
	}

	th, td {
		text-align: left;
		padding: 6px 12px;
		border-bottom: 1px solid #333;
	}

	th {
		color: #888;
		font-weight: 500;
		font-size: 0.85rem;
	}

	td {
		color: #ccc;
	}

	.mono {
		font-family: monospace;
	}

	.panes-info {
		margin-bottom: 24px;
	}

	.pane-list {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
	}

	.pane-tag {
		background: #2a2a3e;
		padding: 4px 10px;
		border-radius: 4px;
		font-size: 0.8rem;
		color: #aaa;
		font-family: monospace;
	}

	.terminal-section {
		height: 500px;
		display: flex;
		flex-direction: column;
	}

	.terminal-section h2 {
		flex-shrink: 0;
	}
</style>
