<script lang="ts">
	import type { ServiceInfo } from '$lib/api';
	import { startService, stopService, reloadService } from '$lib/api';

	let {
		service,
		onUpdate,
		selected = false,
		onSelect,
	}: {
		service: ServiceInfo;
		onUpdate: () => void;
		selected?: boolean;
		onSelect?: (name: string, checked: boolean) => void;
	} = $props();

	let loading = $state(false);

	async function handleAction(e: Event, action: 'start' | 'stop' | 'reload') {
		e.preventDefault();
		e.stopPropagation();
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
</script>

<div class="row" class:stopped={!service.running}>
	{#if onSelect}
		<label class="check-cell">
			<input
				type="checkbox"
				checked={selected}
				onchange={(e) => onSelect?.(service.name, (e.target as HTMLInputElement).checked)}
			/>
		</label>
	{/if}

	<span class="dot-cell">
		<span class="dot" class:running={service.running}>
			<span class="dot-inner"></span>
		</span>
	</span>

	<a href="/service/{service.name}" class="name">{service.name}</a>

	<span class="actions">
		{#if service.running}
			<button
				class="action-btn stop"
				onclick={(e) => handleAction(e, 'stop')}
				disabled={loading}
				title="Stop"
			>
				<svg viewBox="0 0 16 16" fill="currentColor"><rect x="3" y="3" width="10" height="10" rx="1.5" /></svg>
			</button>
			<button
				class="action-btn reload"
				onclick={(e) => handleAction(e, 'reload')}
				disabled={loading}
				title="Reload"
			>
				<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M2.5 8a5.5 5.5 0 0 1 9.9-3.2M13.5 8a5.5 5.5 0 0 1-9.9 3.2" /><polyline points="12 2 13 5 10 5.5" /><polyline points="4 14 3 11 6 10.5" /></svg>
			</button>
			<a
				href="/service/{service.name}"
				class="action-btn echo"
				title="Echo"
			>
				<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 6 2 8 4 10" /><polyline points="12 6 14 8 12 10" /><rect x="1" y="2" width="14" height="12" rx="2" fill="none" /></svg>
			</a>
		{:else}
			<button
				class="action-btn start"
				onclick={(e) => handleAction(e, 'start')}
				disabled={loading}
				title="Start"
			>
				<svg viewBox="0 0 16 16" fill="currentColor"><path d="M4 2.5v11l9-5.5z" /></svg>
			</button>
		{/if}
	</span>

	<span class="path">{shortenPath(service.dir)}</span>
</div>

<style>
	.row {
		display: contents;
	}

	.row > :global(*) {
		padding: var(--row-py, 14px) 0;
		border-bottom: 1px solid #1a1a2a;
		display: flex;
		align-items: center;
		transition: background 0.1s;
	}

	.row:hover > :global(*) {
		background: #13132266;
	}

	.check-cell {
		justify-content: center;
		padding-left: var(--row-px, 16px);
		cursor: pointer;
	}

	.check-cell input {
		width: 15px;
		height: 15px;
		accent-color: #6366f1;
		cursor: pointer;
	}

	.dot-cell {
		justify-content: center;
		padding-left: var(--row-px, 16px);
	}

	.dot {
		width: var(--dot-size, 14px);
		height: var(--dot-size, 14px);
		border-radius: 50%;
		background: radial-gradient(circle at 35% 35%, #ee5555, #aa2222);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		box-shadow: 0 1px 3px #00000044;
	}

	.dot.running {
		background: radial-gradient(circle at 35% 35%, #55ee55, #22aa22);
		box-shadow: 0 0 8px #44bb4455, 0 1px 3px #00000044;
		animation: pulse 3s ease-in-out infinite;
	}

	.dot-inner {
		width: 35%;
		height: 35%;
		border-radius: 50%;
		background: radial-gradient(circle, #ffffff88, transparent);
	}

	@keyframes pulse {
		0%, 100% { box-shadow: 0 0 6px #44bb4433, 0 1px 3px #00000044; }
		50% { box-shadow: 0 0 14px #44bb4466, 0 1px 3px #00000044; }
	}

	.name {
		font-size: var(--name-size, 1.15rem);
		font-weight: 600;
		color: #e8e8e8;
		text-decoration: none;
		white-space: nowrap;
		padding-left: 12px;
		padding-right: 20px;
	}

	.name:hover {
		color: #fff;
	}

	.actions {
		gap: 8px;
		white-space: nowrap;
		padding-right: 20px;
	}

	.action-btn {
		width: var(--btn-size, 32px);
		height: var(--btn-size, 32px);
		border: 1px solid #2a2a3e;
		border-radius: 6px;
		background: #1c1c30;
		color: #777;
		cursor: pointer;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 0;
		text-decoration: none;
		transition: all 0.15s;
	}

	.action-btn svg {
		width: 55%;
		height: 55%;
	}

	.action-btn:hover {
		border-color: #444;
		color: #ccc;
		background: #252540;
	}

	.action-btn.start:hover {
		border-color: #2d5a2d;
		color: #6bdd6b;
		background: #1a2e1a;
	}

	.action-btn.stop:hover {
		border-color: #5a2d2d;
		color: #dd6b6b;
		background: #2e1a1a;
	}

	.action-btn.reload:hover {
		border-color: #3d3d6a;
		color: #8888dd;
		background: #1e1e3a;
	}

	.action-btn.echo:hover {
		border-color: #3d5a3d;
		color: #88cc88;
		background: #1a2a1a;
	}

	.action-btn:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.path {
		font-size: var(--path-size, 0.85rem);
		color: #444;
		font-family: 'SF Mono', Menlo, Monaco, 'Courier New', monospace;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		padding-right: var(--row-px, 16px);
	}

	@media (max-width: 500px) {
		.path {
			display: none !important;
		}
	}
</style>
