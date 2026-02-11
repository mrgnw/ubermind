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
	let innerWidth = $state(800);

	let name = $derived(page.params.name!);
	let scale = $derived(Math.max(0.85, Math.min(1.4, innerWidth / 900)));

	function shortenPath(dir: string): string {
		const home = '/Users/';
		if (dir.startsWith(home)) {
			const rest = dir.slice(home.length);
			const slash = rest.indexOf('/');
			if (slash !== -1) return '~' + rest.slice(slash);
			return '~';
		}
		return dir;
	}

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

<svelte:window bind:innerWidth />

<div
	class="page"
	style:--scale={scale}
	style:--icon-size="{Math.round(22 * scale)}px"
>
	<header>
		<a href="/" class="back" title="Back to services">
			<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M10 3L5 8l5 5" /></svg>
		</a>
		<span class="dot" class:running={detail?.running ?? false}></span>
		<h1>{name}</h1>
		{#if detail}
			<span class="actions">
				{#if detail.running}
					<button class="icon stop" onclick={() => handleAction('stop')} disabled={loading} title="Stop">
						<svg viewBox="0 0 16 16" fill="currentColor"><rect x="3" y="3" width="10" height="10" rx="1.5" /></svg>
					</button>
					<button class="icon reload" onclick={() => handleAction('reload')} disabled={loading} title="Reload">
						<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M2.5 8a5.5 5.5 0 0 1 9.9-3.2M13.5 8a5.5 5.5 0 0 1-9.9 3.2" /><polyline points="12 2 13 5 10 5.5" /><polyline points="4 14 3 11 6 10.5" /></svg>
					</button>
				{:else}
					<button class="icon start" onclick={() => handleAction('start')} disabled={loading} title="Start">
						<svg viewBox="0 0 16 16" fill="currentColor"><path d="M4 2.5v11l9-5.5z" /></svg>
					</button>
				{/if}
			</span>
			<span class="path">{shortenPath(detail.dir)}</span>
		{/if}
	</header>

	{#if error}
		<div class="error">{error}</div>
	{/if}

	{#if detail}
		{#if detail.processes.length > 0}
			<section>
				<h2>processes</h2>
				<div class="proc-list">
					{#each detail.processes as proc}
						<div class="proc-row">
							<span class="proc-dot" class:running={proc.status === 'running'}></span>
							<span class="proc-name">{proc.name}</span>
							<span class="proc-pid">{proc.pid ?? '-'}</span>
						</div>
					{/each}
				</div>
			</section>
		{/if}

		{#if panes.length > 0}
			<section>
				<h2>tmux panes</h2>
				<div class="pane-list">
					{#each panes as pane}
						<span class="pane-tag">
							{pane.session}:{pane.window}.{pane.pane} ({pane.command})
						</span>
					{/each}
				</div>
			</section>
		{/if}

		{#if detail.running && wsUrl}
			<section class="terminal-section">
				<h2>echo</h2>
				<div class="terminal-container">
					<Terminal {wsUrl} />
				</div>
			</section>
		{/if}
	{/if}
</div>

<style>
	.page {
		padding: calc(16px * var(--scale, 1)) calc(20px * var(--scale, 1));
		max-width: 1400px;
		margin: 0 auto;
	}

	header {
		display: flex;
		align-items: center;
		gap: calc(12px * var(--scale, 1));
		margin-bottom: calc(24px * var(--scale, 1));
		flex-wrap: wrap;
	}

	.back {
		color: #555;
		text-decoration: none;
		display: flex;
		align-items: center;
		padding: 0;
		transition: color 0.15s;
	}

	.back svg {
		width: var(--icon-size, 22px);
		height: var(--icon-size, 22px);
	}

	.back:hover {
		color: #ccc;
	}

	.dot {
		width: calc(14px * var(--scale, 1));
		height: calc(14px * var(--scale, 1));
		border-radius: 50%;
		background: #cc4444;
		flex-shrink: 0;
	}

	.dot.running {
		background: #44bb44;
	}

	h1 {
		font-size: calc(1.3rem * var(--scale, 1));
		font-weight: 700;
		color: #e0e0e0;
		margin: 0;
	}

	h2 {
		font-size: calc(0.8rem * var(--scale, 1));
		font-weight: 500;
		color: #555;
		margin: 0 0 calc(8px * var(--scale, 1));
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	section {
		margin-bottom: calc(24px * var(--scale, 1));
	}

	.actions {
		display: flex;
		gap: calc(10px * var(--scale, 1));
	}

	.icon {
		width: var(--icon-size, 22px);
		height: var(--icon-size, 22px);
		border: none;
		background: none;
		color: #555;
		cursor: pointer;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 0;
		transition: color 0.15s;
	}

	.icon svg {
		width: 100%;
		height: 100%;
	}

	.icon:hover { color: #ccc; }
	.icon.start:hover { color: #55cc55; }
	.icon.stop:hover { color: #dd6666; }
	.icon.reload:hover { color: #7777cc; }
	.icon:disabled { opacity: 0.25; cursor: not-allowed; }

	.path {
		font-size: calc(0.8rem * var(--scale, 1));
		color: #3a3a4a;
		font-family: 'SF Mono', Menlo, Monaco, 'Courier New', monospace;
		margin-left: auto;
	}

	.error {
		background: #2a1010;
		border: 1px solid #552222;
		border-radius: 6px;
		padding: 12px 16px;
		margin-bottom: 16px;
		color: #cc6666;
		font-size: 0.9rem;
	}

	.proc-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.proc-row {
		display: flex;
		align-items: center;
		gap: calc(10px * var(--scale, 1));
		padding: calc(6px * var(--scale, 1)) 0;
		border-bottom: 1px solid #1a1a28;
	}

	.proc-dot {
		width: calc(9px * var(--scale, 1));
		height: calc(9px * var(--scale, 1));
		border-radius: 50%;
		background: #cc4444;
		flex-shrink: 0;
	}

	.proc-dot.running {
		background: #44bb44;
	}

	.proc-name {
		font-weight: 500;
		color: #ccc;
		min-width: 80px;
		font-size: calc(1rem * var(--scale, 1));
	}

	.proc-pid {
		font-size: calc(0.8rem * var(--scale, 1));
		color: #555;
		font-family: 'SF Mono', Menlo, Monaco, 'Courier New', monospace;
	}

	.pane-list {
		display: flex;
		gap: 6px;
		flex-wrap: wrap;
	}

	.pane-tag {
		background: #161628;
		padding: 3px 8px;
		border-radius: 3px;
		font-size: calc(0.75rem * var(--scale, 1));
		color: #666;
		font-family: 'SF Mono', Menlo, Monaco, 'Courier New', monospace;
	}

	.terminal-section {
		display: flex;
		flex-direction: column;
		min-height: 0;
	}

	.terminal-container {
		flex: 1;
		height: calc(100vh - 280px);
		min-height: 250px;
	}
</style>
