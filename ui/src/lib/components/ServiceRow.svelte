<script lang="ts">
	import { goto } from '$app/navigation';
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
</script>

<a href="/service/{service.name}" class="row" class:stopped={!service.running}>
	<span class="dot-cell">
		{#if onSelect}
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<label class="check-wrap" onclick={(e: MouseEvent) => e.stopPropagation()}>
				<input
					type="checkbox"
					checked={selected}
					onchange={(e) => onSelect?.(service.name, (e.target as HTMLInputElement).checked)}
				/>
			</label>
		{:else}
			<span class="dot" class:running={service.running}></span>
		{/if}
	</span>

	<span class="name">{service.name}</span>

	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<span class="actions" onclick={(e: MouseEvent) => e.preventDefault()}>
		{#if service.running}
			<button class="icon stop" onclick={(e) => handleAction(e, 'stop')} disabled={loading} title="Stop">
				<svg viewBox="0 0 16 16" fill="currentColor"><rect x="3" y="3" width="10" height="10" rx="1.5" /></svg>
			</button>
			<button class="icon reload" onclick={(e) => handleAction(e, 'reload')} disabled={loading} title="Reload">
				<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M2.5 8a5.5 5.5 0 0 1 9.9-3.2M13.5 8a5.5 5.5 0 0 1-9.9 3.2" /><polyline points="12 2 13 5 10 5.5" /><polyline points="4 14 3 11 6 10.5" /></svg>
			</button>
			<button class="icon echo" title="Echo" onclick={(e) => { e.preventDefault(); e.stopPropagation(); goto(`/service/${service.name}`); }}>
				<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 6 2 8 4 10" /><polyline points="12 6 14 8 12 10" /><rect x="1" y="2" width="14" height="12" rx="2" fill="none" /></svg>
			</button>
		{:else}
			<button class="icon start" onclick={(e) => handleAction(e, 'start')} disabled={loading} title="Start">
				<svg viewBox="0 0 16 16" fill="currentColor"><path d="M4 2.5v11l9-5.5z" /></svg>
			</button>
		{/if}
	</span>
</a>

<style>
	.row {
		display: grid;
		grid-template-columns: subgrid;
		grid-column: 1 / -1;
		align-items: center;
		border-bottom: 1px solid #1a1a2a;
		text-decoration: none;
		transition: background 0.1s;
	}

	.row:hover {
		background: #12122066;
	}

	.dot-cell {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--row-py, 16px) 0;
	}

	.check-wrap {
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
	}

	.check-wrap input {
		width: calc(18px * var(--scale, 1));
		height: calc(18px * var(--scale, 1));
		accent-color: #6366f1;
		cursor: pointer;
	}

	.dot {
		width: var(--dot-size, 18px);
		height: var(--dot-size, 18px);
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
		white-space: nowrap;
		padding: var(--row-py, 16px) 0;
	}

	.row:hover .name {
		color: #fff;
	}

	.actions {
		display: flex;
		align-items: center;
		gap: var(--icon-gap, 14px);
		white-space: nowrap;
		padding: var(--row-py, 16px) 0;
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
</style>
