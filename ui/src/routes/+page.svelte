<script lang="ts">
	import { onMount } from 'svelte';
	import { getServices, startService, stopService, reloadService, type ServiceInfo } from '$lib/api';
	import ServiceRow from '$lib/components/ServiceRow.svelte';
	import logoSvg from '$lib/assets/logo.svg';

	let services = $state<ServiceInfo[]>([]);
	let error = $state('');
	let refreshTimer: ReturnType<typeof setInterval>;
	let innerWidth = $state(800);
	let selectedNames = $state<Set<string>>(new Set());
	let bulkLoading = $state(false);
	let headerCheckbox = $state<HTMLInputElement | null>(null);

	let scale = $derived(Math.max(0.85, Math.min(1.6, innerWidth / 800)));
	let hasSelection = $derived(selectedNames.size > 0);
	let allSelected = $derived(services.length > 0 && selectedNames.size === services.length);
	let someSelected = $derived(hasSelection && !allSelected);
	let runningCount = $derived(services.filter(s => s.running).length);
	let stoppedCount = $derived(services.filter(s => !s.running).length);

	let selectedServices = $derived(services.filter(s => selectedNames.has(s.name)));
	let selectedRunning = $derived(selectedServices.filter(s => s.running).length);
	let selectedStopped = $derived(selectedServices.filter(s => !s.running).length);

	function syncIndeterminate() {
		if (headerCheckbox) {
			headerCheckbox.indeterminate = someSelected;
		}
	}

	async function refresh() {
		try {
			services = await getServices();
			error = '';
			selectedNames = new Set([...selectedNames].filter(n => services.some(s => s.name === n)));
			queueMicrotask(syncIndeterminate);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	function toggleSelect(name: string, checked: boolean) {
		const next = new Set(selectedNames);
		if (checked) next.add(name);
		else next.delete(name);
		selectedNames = next;
		queueMicrotask(syncIndeterminate);
	}

	function headerCheckClicked() {
		if (allSelected || someSelected) {
			selectedNames = new Set();
		} else {
			selectedNames = new Set(services.map(s => s.name));
		}
		queueMicrotask(syncIndeterminate);
	}

	async function bulkAction(action: 'start' | 'stop' | 'reload') {
		bulkLoading = true;
		const targets = [...selectedNames];
		try {
			await Promise.allSettled(
				targets.map(name => {
					if (action === 'start') return startService(name);
					if (action === 'stop') return stopService(name);
					return reloadService(name);
				})
			);
			setTimeout(refresh, 800);
		} catch (e) {
			console.error(e);
		} finally {
			bulkLoading = false;
		}
	}

	async function actionAll(action: 'start' | 'stop') {
		bulkLoading = true;
		const targets = action === 'start'
			? services.filter(s => !s.running)
			: services.filter(s => s.running);
		try {
			await Promise.allSettled(targets.map(s =>
				action === 'start' ? startService(s.name) : stopService(s.name)
			));
			setTimeout(refresh, 800);
		} catch (e) {
			console.error(e);
		} finally {
			bulkLoading = false;
		}
	}

	onMount(() => {
		refresh();
		refreshTimer = setInterval(refresh, 5000);
		return () => clearInterval(refreshTimer);
	});
</script>

<svelte:window bind:innerWidth />

<div
	class="page"
	style:--scale={scale}
	style:--row-py="{Math.round(18 * scale)}px"
	style:--dot-size="{Math.round(18 * scale)}px"
	style:--name-size="{1.35 * scale}rem"
	style:--icon-size="{Math.round(28 * scale)}px"
	style:--icon-gap="{Math.round(14 * scale)}px"
>
	<div class="logo-banner">
		<img src={logoSvg} alt="ubermind" class="logo" />
	</div>

	{#if error}
		<div class="error">
			{error}
			<p>Make sure the ubermind-ui server is running on port 13369</p>
		</div>
	{/if}

	<div class="list">
		<div class="header-row">
			<span class="header-dot-cell">
				<label class="header-check">
					<input
						type="checkbox"
						bind:this={headerCheckbox}
						checked={allSelected}
						onclick={headerCheckClicked}
					/>
				</label>
			</span>
			<span class="header-name">ubermind</span>
			<span class="header-actions">
				{#if hasSelection}
					{#if selectedStopped > 0}
						<button class="hicon start" onclick={() => bulkAction('start')} disabled={bulkLoading} title="Start selected">
							<svg viewBox="0 0 16 16" fill="currentColor"><path d="M4 2.5v11l9-5.5z" /></svg>
						</button>
					{/if}
					{#if selectedRunning > 0}
						<button class="hicon stop" onclick={() => bulkAction('stop')} disabled={bulkLoading} title="Stop selected">
							<svg viewBox="0 0 16 16" fill="currentColor"><rect x="3" y="3" width="10" height="10" rx="1.5" /></svg>
						</button>
						<button class="hicon reload" onclick={() => bulkAction('reload')} disabled={bulkLoading} title="Reload selected">
							<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M2.5 8a5.5 5.5 0 0 1 9.9-3.2M13.5 8a5.5 5.5 0 0 1-9.9 3.2" /><polyline points="12 2 13 5 10 5.5" /><polyline points="4 14 3 11 6 10.5" /></svg>
						</button>
					{/if}
				{:else}
					{#if stoppedCount > 0}
						<button class="hicon start" onclick={() => actionAll('start')} disabled={bulkLoading} title="Start all">
							<svg viewBox="0 0 16 16" fill="currentColor"><path d="M4 2.5v11l9-5.5z" /></svg>
						</button>
					{/if}
					{#if runningCount > 0}
						<button class="hicon stop" onclick={() => actionAll('stop')} disabled={bulkLoading} title="Stop all">
							<svg viewBox="0 0 16 16" fill="currentColor"><rect x="3" y="3" width="10" height="10" rx="1.5" /></svg>
						</button>
					{/if}
				{/if}
				<span class="summary">
					{runningCount} running{#if stoppedCount > 0}, {stoppedCount} stopped{/if}
				</span>
			</span>
		</div>

		{#each services as service (service.name)}
			<ServiceRow
				{service}
				onUpdate={refresh}
				selected={selectedNames.has(service.name)}
				onSelect={hasSelection ? toggleSelect : undefined}
			/>
		{/each}
	</div>

	{#if services.length === 0 && !error}
		<p class="empty">No services configured. Run <code>ubermind init</code> to get started.</p>
	{/if}
</div>

<style>
	.page {
		padding: calc(12px * var(--scale, 1)) calc(28px * var(--scale, 1));
		max-width: 1400px;
		margin: 0 auto;
	}

	.logo-banner {
		display: flex;
		justify-content: center;
		padding: calc(28px * var(--scale, 1)) 0 calc(20px * var(--scale, 1));
	}

	.logo {
		width: calc(72px * var(--scale, 1));
		height: calc(72px * var(--scale, 1));
		opacity: 0.6;
		filter: brightness(0) invert(1);
	}

	.list {
		display: grid;
		grid-template-columns: calc(60px * var(--scale, 1)) auto 1fr;
		align-items: center;
	}

	.header-row {
		display: grid;
		grid-template-columns: subgrid;
		grid-column: 1 / -1;
		align-items: center;
		border-bottom: 1px solid #222238;
	}

	.header-dot-cell {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: calc(24px * var(--scale, 1)) 0;
	}

	.header-check {
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
	}

	.header-check input {
		width: calc(20px * var(--scale, 1));
		height: calc(20px * var(--scale, 1));
		accent-color: #6366f1;
		cursor: pointer;
	}

	.header-name {
		font-size: calc(1.7rem * var(--scale, 1));
		font-weight: 700;
		color: #555;
		padding: calc(24px * var(--scale, 1)) 0;
	}

	.header-actions {
		display: flex;
		align-items: center;
		gap: calc(16px * var(--scale, 1));
		padding: calc(24px * var(--scale, 1)) 0;
		justify-content: flex-end;
	}

	.summary {
		font-size: calc(1rem * var(--scale, 1));
		color: #3a3a4a;
		font-family: 'SF Mono', Menlo, Monaco, 'Courier New', monospace;
	}

	.hicon {
		width: calc(32px * var(--scale, 1));
		height: calc(32px * var(--scale, 1));
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

	.hicon svg {
		width: 100%;
		height: 100%;
	}

	.hicon:hover { color: #ccc; }
	.hicon.start:hover { color: #55cc55; }
	.hicon.stop:hover { color: #dd6666; }
	.hicon.reload:hover { color: #7777cc; }
	.hicon:disabled { opacity: 0.25; cursor: not-allowed; }

	.error {
		background: #2a1010;
		border: 1px solid #552222;
		border-radius: 6px;
		padding: 12px 16px;
		margin-bottom: 16px;
		color: #cc6666;
		font-size: calc(1rem * var(--scale, 1));
	}

	.error p {
		margin: 6px 0 0;
		font-size: calc(0.9rem * var(--scale, 1));
		color: #888;
	}

	.empty {
		color: #555;
		text-align: center;
		margin-top: 48px;
		font-size: calc(1rem * var(--scale, 1));
	}

	code {
		background: #1a1a2e;
		padding: 2px 6px;
		border-radius: 3px;
		font-size: calc(0.95rem * var(--scale, 1));
	}
</style>
