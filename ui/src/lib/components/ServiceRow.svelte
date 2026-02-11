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
		<span class="dot" class:running={service.running}></span>
	</span>

	<a href="/service/{service.name}" class="name">{service.name}</a>

	<span class="actions">
		{#if service.running}
			<button class="icon stop" onclick={(e) => handleAction(e, 'stop')} disabled={loading} title="Stop">
				<svg viewBox="0 0 16 16" fill="currentColor"><rect x="3" y="3" width="10" height="10" rx="1.5" /></svg>
			</button>
			<button class="icon reload" onclick={(e) => handleAction(e, 'reload')} disabled={loading} title="Reload">
				<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M2.5 8a5.5 5.5 0 0 1 9.9-3.2M13.5 8a5.5 5.5 0 0 1-9.9 3.2" /><polyline points="12 2 13 5 10 5.5" /><polyline points="4 14 3 11 6 10.5" /></svg>
			</button>
			<a href="/service/{service.name}" class="icon echo" title="Echo">
				<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 6 2 8 4 10" /><polyline points="12 6 14 8 12 10" /><rect x="1" y="2" width="14" height="12" rx="2" fill="none" /></svg>
			</a>
		{:else}
			<button class="icon start" onclick={(e) => handleAction(e, 'start')} disabled={loading} title="Start">
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
		background: #12122066;
	}

	.check-cell {
		justify-content: center;
		padding-left: var(--row-px, 16px);
		cursor: pointer;
	}

	.check-cell input {
		width: calc(18px * var(--scale, 1));
		height: calc(18px * var(--scale, 1));
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
		background: #cc4444;
		flex-shrink: 0;
	}

	.dot.running {
		background: #44bb44;
	}

	.name {
		font-size: var(--name-size, 1.3rem);
		font-weight: 600;
		color: #e0e0e0;
		text-decoration: none;
		white-space: nowrap;
		padding-left: calc(14px * var(--scale, 1));
		padding-right: calc(24px * var(--scale, 1));
	}

	.name:hover {
		color: #fff;
	}

	.actions {
		gap: var(--icon-gap, 14px);
		white-space: nowrap;
		padding-right: calc(24px * var(--scale, 1));
	}

	.icon {
		width: var(--icon-size, 28px);
		height: var(--icon-size, 28px);
		border: none;
		background: none;
		color: #555;
		cursor: pointer;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 0;
		text-decoration: none;
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
	.icon.echo:hover { color: #66aa88; }
	.icon:disabled { opacity: 0.25; cursor: not-allowed; }

	.path {
		font-size: var(--path-size, 0.95rem);
		color: #3a3a4a;
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
