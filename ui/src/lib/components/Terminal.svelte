<script lang="ts">
	import { onMount } from 'svelte';
	import { Terminal } from '@xterm/xterm';
	import { FitAddon } from '@xterm/addon-fit';
	import '@xterm/xterm/css/xterm.css';

	let { content = '', wsUrl = '' }: { content?: string; wsUrl?: string } = $props();

	let container: HTMLDivElement;
	let term: Terminal;
	let fitAddon: FitAddon;
	let ws: WebSocket | null = null;

	onMount(() => {
		term = new Terminal({
			convertEol: true,
			scrollback: 5000,
			fontSize: 13,
			fontFamily: 'Menlo, Monaco, "Courier New", monospace',
			theme: {
				background: '#1a1a2e',
				foreground: '#e0e0e0',
				cursor: '#e0e0e0',
				selectionBackground: '#44475a',
			},
		});

		fitAddon = new FitAddon();
		term.loadAddon(fitAddon);
		term.open(container);
		fitAddon.fit();

		if (content) {
			term.write(content);
		}

		if (wsUrl) {
			connectWs();
		}

		const resizeObserver = new ResizeObserver(() => {
			fitAddon.fit();
		});
		resizeObserver.observe(container);

		return () => {
			resizeObserver.disconnect();
			ws?.close();
			term.dispose();
		};
	});

	function connectWs() {
		if (!wsUrl) return;
		ws = new WebSocket(wsUrl);
		ws.onmessage = (event) => {
			term.clear();
			term.write(event.data);
		};
		ws.onclose = () => {
			setTimeout(connectWs, 1000);
		};
		ws.onerror = () => {
			ws?.close();
		};
	}
</script>

<div class="terminal-wrapper" bind:this={container}></div>

<style>
	.terminal-wrapper {
		width: 100%;
		height: 100%;
		min-height: 300px;
	}

	.terminal-wrapper :global(.xterm) {
		padding: 8px;
	}
</style>
