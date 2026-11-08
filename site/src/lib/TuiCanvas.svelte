<!--
	A terminal canvas (#137): the ratatui twin compiled to WASM renders a Forge
	document into a TestBackend frame; we draw the rows as a monospace grid.
	No iframe — it's a pure render_frame() call, unlike the GPUI canvases.
-->
<script lang="ts">
	import { asset } from '$app/paths';
	import { onMount } from 'svelte';
	import type { Diagnostic } from '#lib/Canvas.svelte';

	type Glue = {
		render_frame?: (yaml: string, state: string, w: number, h: number) => string;
		default?: () => Promise<unknown>;
	};

	let {
		yaml,
		bindings = undefined,
		height = '360px',
		ondiagnostics = undefined
	}: {
		yaml: string;
		bindings?: unknown;
		height?: string;
		ondiagnostics?: (d: Diagnostic[]) => void;
	} = $props();

	const COLS = 90;
	const ROWS = 30;

	let rows = $state<string[]>([]);
	let ready = $state(false);
	let missing = $state(false);
	let glue = $state<Glue | undefined>(undefined);

	onMount(async () => {
		try {
			const mod = (await import(
				/* @vite-ignore */ asset('canvas/pkg/ratatui/forge_web_canvas.js')
			)) as Glue;
			await mod.default?.();
			glue = mod;
			ready = true;
		} catch {
			missing = true;
		}
	});

	$effect(() => {
		if (!ready || !glue?.render_frame) return;
		try {
			const out = JSON.parse(glue.render_frame(yaml, JSON.stringify(bindings ?? {}), COLS, ROWS));
			rows = out.rows ?? [];
			ondiagnostics?.(out.diagnostics ?? []);
		} catch {
			rows = [];
		}
	});
</script>

<div class="tui" style:height>
	{#if missing}
		<div class="muted">ratatui canvas not built — run site/scripts/build-canvas.sh</div>
	{:else if !ready}
		<div class="muted">Loading the ratatui canvas…</div>
	{:else}
		<pre>{rows.join('\n')}</pre>
	{/if}
</div>

<style>
	.tui { position: relative; width: 100%; background: #0a0a0a; overflow: auto; }
	pre { margin: 0; padding: 12px; font: 12px/1.2 var(--mono, monospace); color: #d4d4d8; white-space: pre; }
	.muted { display: grid; place-items: center; height: 100%; color: var(--muted); font-size: 13px; }
</style>
