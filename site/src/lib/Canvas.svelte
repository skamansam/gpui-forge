<!--
	A live gpui-kit canvas (crates/forge-web-canvas compiled to WASM). GPUI draws into a
	canvas on <body>, so the canvas runs in an iframe and this component talks to it with
	postMessage; see static/canvas/index.html.
-->
<script lang="ts" module>
	export type Diagnostic = { line: number; col: number; message: string; hint: string | null };
</script>

<script lang="ts">
	import { asset } from '$app/paths';
	import { onMount } from 'svelte';

	let {
		yaml,
		bindings = undefined,
		height = '360px',
		ondiagnostics = undefined,
		onhook = undefined,
		onstate = undefined
	}: {
		yaml: string;
		bindings?: unknown;
		height?: string;
		ondiagnostics?: (d: Diagnostic[]) => void;
		onhook?: (call: string) => void;
		onstate?: (state: unknown) => void;
	} = $props();

	let frame: HTMLIFrameElement;
	let ready = $state(false);

	const post = (message: object) => frame?.contentWindow?.postMessage(message, '*');

	// State replaces the canvas's bindings (and resets its inputs), so it is only sent when it
	// changes; YAML edits re-render against the current state.
	$effect(() => {
		const s = $state.snapshot(bindings);
		if (ready) post({ type: 'forge:render', yaml, state: s ?? {} });
	});
	$effect(() => {
		if (ready) post({ type: 'forge:render', yaml });
	});

	onMount(() => {
		const listen = ({ source, data }: MessageEvent) => {
			if (source !== frame?.contentWindow) return;
			if (data.type === 'forge:ready') ready = true;
			else if (data.type === 'forge:diagnostics') ondiagnostics?.(data.diagnostics);
			else if (data.type === 'forge:hook') onhook?.(data.call);
			else if (data.type === 'forge:state') onstate?.(data.state);
		};
		addEventListener('message', listen);
		// A prerendered iframe can finish loading before this component hydrates and miss its
		// `ready`, so ask until it answers.
		const hello = setInterval(() => (ready ? clearInterval(hello) : post({ type: 'forge:hello' })), 300);
		return () => {
			clearInterval(hello);
			removeEventListener('message', listen);
		};
	});
</script>

<div class="canvas" style:height>
	<iframe bind:this={frame} src={asset('canvas/index.html')} title="gpui-kit canvas"></iframe>
	{#if !ready}<div class="loading">Loading the gpui-kit canvas…</div>{/if}
</div>

<style>
	.canvas { position: relative; width: 100%; background: #0a0a0a; }
	iframe { width: 100%; height: 100%; border: 0; display: block; }
	.loading { position: absolute; inset: 0; display: grid; place-items: center; color: var(--muted); font-size: 13px; }
</style>
