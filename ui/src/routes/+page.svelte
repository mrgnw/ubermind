<script lang="ts">
	import { onMount } from 'svelte';
	import { getServices, startService, stopService, reloadService, type ServiceInfo } from '$lib/api';
	import ServiceRow from '$lib/components/ServiceRow.svelte';

	let services = $state<ServiceInfo[]>([]);
	let error = $state('');
	let refreshTimer: ReturnType<typeof setInterval>;
	let innerWidth = $state(800);
	let innerHeight = $state(600);
	let selectedNames = $state<Set<string>>(new Set());
	let bulkMode = $state(false);
	let bulkLoading = $state(false);

	let scale = $derived(Math.max(0.85, Math.min(1.4, innerWidth / 900)));
	let hasSelection = $derived(selectedNames.size > 0);
	let allSelected = $derived(services.length > 0 && selectedNames.size === services.length);
	let runningCount = $derived(services.filter(s => s.running).length);
	let stoppedCount = $derived(services.filter(s => !s.running).length);

	let selectedServices = $derived(
		services.filter(s => selectedNames.has(s.name))
	);
	let selectedRunning = $derived(selectedServices.filter(s => s.running).length);
	let selectedStopped = $derived(selectedServices.filter(s => !s.running).length);

	async function refresh() {
		try {
			services = await getServices();
			error = '';
			selectedNames = new Set([...selectedNames].filter(n => services.some(s => s.name === n)));
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	function toggleSelect(name: string, checked: boolean) {
		const next = new Set(selectedNames);
		if (checked) next.add(name);
		else next.delete(name);
		selectedNames = next;
	}

	function toggleAll() {
		if (allSelected) {
			selectedNames = new Set();
		} else {
			selectedNames = new Set(services.map(s => s.name));
		}
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

	async function bulkStartAll() {
		bulkLoading = true;
		const stopped = services.filter(s => !s.running);
		try {
			await Promise.allSettled(stopped.map(s => startService(s.name)));
			setTimeout(refresh, 800);
		} catch (e) {
			console.error(e);
		} finally {
			bulkLoading = false;
		}
	}

	async function bulkStopAll() {
		bulkLoading = true;
		const running = services.filter(s => s.running);
		try {
			await Promise.allSettled(running.map(s => stopService(s.name)));
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

<svelte:window bind:innerWidth bind:innerHeight />

<div
	class="page"
	style:--scale={scale}
	style:--row-py="{Math.round(12 * scale)}px"
	style:--row-px="{Math.round(16 * scale)}px"
	style:--dot-size="{Math.round(14 * scale)}px"
	style:--name-size="{1.1 * scale}rem"
	style:--btn-size="{Math.round(32 * scale)}px"
	style:--path-size="{0.82 * scale}rem"
>
	<header>
		<h1>ubermind</h1>
		<div class="header-actions">
			<span class="summary">
				{runningCount} running{#if stoppedCount > 0}, {stoppedCount} stopped{/if}
			</span>
			<button class="header-btn" onclick={() => { bulkMode = !bulkMode; if (!bulkMode) selectedNames = new Set(); }}>
				{bulkMode ? 'done' : 'select'}
			</button>
			{#if !bulkMode}
				{#if stoppedCount > 0}
					<button class="header-btn start-all" onclick={bulkStartAll} disabled={bulkLoading}>
						<svg viewBox="0 0 16 16" fill="currentColor" width="12" height="12"><path d="M4 2.5v11l9-5.5z" /></svg>
						start all
					</button>
				{/if}
				{#if runningCount > 0}
					<button class="header-btn stop-all" onclick={bulkStopAll} disabled={bulkLoading}>
						<svg viewBox="0 0 16 16" fill="currentColor" width="12" height="12"><rect x="3" y="3" width="10" height="10" rx="1.5" /></svg>
						stop all
					</button>
				{/if}
			{/if}
		</div>
	</header>

	{#if error}
		<div class="error">
			{error}
			<p>Make sure the ubermind-ui server is running on port 13369</p>
		</div>
	{/if}

	{#if bulkMode && hasSelection}
		<div class="bulk-bar">
			<span class="bulk-count">{selectedNames.size} selected</span>
			{#if selectedStopped > 0}
				<button class="bulk-btn start" onclick={() => bulkAction('start')} disabled={bulkLoading}>
					<svg viewBox="0 0 16 16" fill="currentColor" width="14" height="14"><path d="M4 2.5v11l9-5.5z" /></svg>
					start
				</button>
			{/if}
			{#if selectedRunning > 0}
				<button class="bulk-btn stop" onclick={() => bulkAction('stop')} disabled={bulkLoading}>
					<svg viewBox="0 0 16 16" fill="currentColor" width="14" height="14"><rect x="3" y="3" width="10" height="10" rx="1.5" /></svg>
					stop
				</button>
				<button class="bulk-btn reload" onclick={() => bulkAction('reload')} disabled={bulkLoading}>
					<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" width="14" height="14"><path d="M2.5 8a5.5 5.5 0 0 1 9.9-3.2M13.5 8a5.5 5.5 0 0 1-9.9 3.2" /><polyline points="12 2 13 5 10 5.5" /><polyline points="4 14 3 11 6 10.5" /></svg>
					reload
				</button>
			{/if}
		</div>
	{/if}

	<div class="list" class:with-check={bulkMode}>
		{#if bulkMode}
			<label class="select-all-cell">
				<input type="checkbox" checked={allSelected} onchange={toggleAll} />
			</label>
		{/if}
		<!-- column headers occupy 1 grid row to avoid visual shift -->
		<span class="col-header dot-col"></span>
		<span class="col-header name-col"></span>
		<span class="col-header actions-col"></span>
		<span class="col-header path-col"></span>

		{#each services as service (service.name)}
			<ServiceRow
				{service}
				onUpdate={refresh}
				selected={selectedNames.has(service.name)}
				onSelect={bulkMode ? toggleSelect : undefined}
			/>
		{/each}
	</div>

	{#if services.length === 0 && !error}
		<p class="empty">No services configured. Run <code>ubermind init</code> to get started.</p>
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
		justify-content: space-between;
		margin-bottom: calc(14px * var(--scale, 1));
		flex-wrap: wrap;
		gap: 12px;
	}

	h1 {
		font-size: calc(1.3rem * var(--scale, 1));
		font-weight: 700;
		color: #ccc;
		margin: 0;
		letter-spacing: 0.02em;
	}

	.header-actions {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.summary {
		font-size: 0.8rem;
		color: #555;
	}

	.header-btn {
		padding: 5px 12px;
		border: 1px solid #2a2a3e;
		border-radius: 5px;
		background: #1a1a2e;
		color: #888;
		cursor: pointer;
		font-size: 0.8rem;
		font-family: inherit;
		display: inline-flex;
		align-items: center;
		gap: 5px;
		transition: all 0.15s;
	}

	.header-btn:hover {
		border-color: #444;
		color: #ccc;
		background: #222240;
	}

	.header-btn.start-all:hover {
		border-color: #2d5a2d;
		color: #6bdd6b;
	}

	.header-btn.stop-all:hover {
		border-color: #5a2d2d;
		color: #dd6b6b;
	}

	.header-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.bulk-bar {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 16px;
		margin-bottom: 8px;
		background: #161630;
		border: 1px solid #2a2a4a;
		border-radius: 6px;
	}

	.bulk-count {
		font-size: 0.85rem;
		color: #999;
		margin-right: auto;
	}

	.bulk-btn {
		padding: 5px 14px;
		border: 1px solid #333;
		border-radius: 5px;
		background: #1c1c34;
		color: #aaa;
		cursor: pointer;
		font-size: 0.8rem;
		font-family: inherit;
		display: inline-flex;
		align-items: center;
		gap: 6px;
		transition: all 0.15s;
	}

	.bulk-btn:hover {
		background: #252545;
		border-color: #555;
		color: #ddd;
	}

	.bulk-btn.start:hover { color: #6bdd6b; border-color: #2d5a2d; }
	.bulk-btn.stop:hover { color: #dd6b6b; border-color: #5a2d2d; }
	.bulk-btn.reload:hover { color: #8888dd; border-color: #3d3d6a; }
	.bulk-btn:disabled { opacity: 0.4; cursor: not-allowed; }

	.list {
		display: grid;
		grid-template-columns: 40px auto auto minmax(0, 1fr);
		align-items: center;
		border-top: 1px solid #1a1a2a;
	}

	.list.with-check {
		grid-template-columns: 36px 40px auto auto minmax(0, 1fr);
	}

	.select-all-cell {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 8px 0;
		border-bottom: 1px solid #1a1a2a;
		cursor: pointer;
		padding-left: var(--row-px, 16px);
	}

	.select-all-cell input {
		width: 15px;
		height: 15px;
		accent-color: #6366f1;
		cursor: pointer;
	}

	.col-header {
		padding: 0;
		border-bottom: 1px solid #1a1a2a;
		height: 1px;
	}

	@media (max-width: 500px) {
		.list {
			grid-template-columns: 40px auto auto;
		}
		.list.with-check {
			grid-template-columns: 36px 40px auto auto;
		}
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

	.error p {
		margin: 6px 0 0;
		font-size: 0.8rem;
		color: #888;
	}

	.empty {
		color: #555;
		text-align: center;
		margin-top: 48px;
	}

	code {
		background: #1a1a2e;
		padding: 2px 6px;
		border-radius: 3px;
		font-size: 0.85rem;
	}
</style>
