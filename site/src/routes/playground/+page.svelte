<script lang="ts">
	import { onMount } from 'svelte';
	import Canvas, { type Diagnostic } from '#lib/Canvas.svelte';
	import YamlEditor from '#lib/YamlEditor.svelte';
	import Diagnostics from '#lib/Diagnostics.svelte';
	import { examples } from '#lib/examples.ts';

	let yaml = $state(examples[0].yaml);
	let stateText = $state(JSON.stringify(examples[0].state, null, 2));
	let bindings = $state<unknown>(examples[0].state);
	let liveState = $state<unknown>(examples[0].state);
	let diagnostics = $state<Diagnostic[]>([]);
	let hooks = $state<string[]>([]);
	let stateError = $state('');
	let copied = $state(false);

	const encode = (s: string) => btoa(String.fromCharCode(...new TextEncoder().encode(s)));
	const decode = (s: string) => new TextDecoder().decode(Uint8Array.from(atob(s), (c) => c.charCodeAt(0)));

	onMount(() => {
		if (location.hash.length > 1) {
			try {
				const shared = JSON.parse(decode(location.hash.slice(1)));
				yaml = shared.yaml;
				if (shared.state !== undefined) {
					stateText = JSON.stringify(shared.state, null, 2);
					applyState();
				}
			} catch { /* keep the default */ }
		}
	});

	function load(i: number) {
		yaml = examples[i].yaml;
		stateText = JSON.stringify(examples[i].state, null, 2);
		applyState();
	}
	function applyState() {
		try { bindings = JSON.parse(stateText); liveState = bindings; stateError = ''; }
		catch (e) { stateError = (e as Error).message; }
	}
	async function share() {
		history.replaceState(null, '', `#${encode(JSON.stringify({ yaml, state: liveState }))}`);
		await navigator.clipboard?.writeText(location.href);
		copied = true;
		setTimeout(() => (copied = false), 1500);
	}
</script>

<svelte:head><title>Playground — GPUI Forge</title></svelte:head>

<div class="bar">
	<strong>Playground</strong>
	<span class="muted">Load an example:</span>
	{#each examples as ex, i (ex.slug)}<button class="btn small" onclick={() => load(i)}>{ex.title}</button>{/each}
	<span class="spacer"></span>
	<button class="btn small primary" onclick={share}>{copied ? 'Link copied' : 'Share link'}</button>
</div>

<div class="grid">
	<div class="panel col">
		<div class="panel-title"><span>document.forge.yaml</span><span>{diagnostics.length} problem{diagnostics.length === 1 ? '' : 's'}</span></div>
		<YamlEditor bind:value={yaml} height="calc(100vh - 290px)" />
		<Diagnostics {diagnostics} />
	</div>
	<div class="panel col">
		<div class="panel-title"><span>gpui-kit canvas</span></div>
		<Canvas {yaml} {bindings} height="calc(100vh - 290px)" ondiagnostics={(d) => (diagnostics = d)}
			onhook={(c) => (hooks = [...hooks.slice(-4), c])} onstate={(s) => (liveState = s)} />
		<div class="hooks">
			{#each hooks as h, i (i)}<div>hook fired: <code>{h}</code></div>{:else}<div class="muted">Click a button with an <code>on click:</code> handler to see hooks fire.</div>{/each}
		</div>
	</div>
	<div class="panel col">
		<div class="panel-title"><span>view state (JSON)</span><button class="btn small" onclick={applyState}>Apply</button></div>
		<textarea bind:value={stateText} spellcheck="false"></textarea>
		{#if stateError}<div class="err">{stateError}</div>{/if}
		<div class="panel-title"><span>live state</span></div>
		<pre class="live">{JSON.stringify(liveState, null, 2)}</pre>
	</div>
</div>

<style>
	.bar { display: flex; align-items: center; gap: 10px; padding: 12px 24px; flex-wrap: wrap; }
	.spacer { flex: 1; }
	.muted { color: var(--muted); font-size: 13px; }
	.btn.small { padding: 4px 10px; font-size: 13px; }
	.grid { display: grid; grid-template-columns: 1.1fr 1.1fr 0.8fr; gap: 12px; padding: 0 24px 24px; }
	.col { display: flex; flex-direction: column; min-width: 0; }
	.hooks { padding: 8px 12px; font-size: 13px; min-height: 140px; border-top: 1px solid var(--border); }
	textarea { background: var(--panel); color: var(--fg); border: 0; font: 13px var(--mono); padding: 12px; min-height: 180px; resize: vertical; outline: none; }
	pre.live { margin: 0; border: 0; border-radius: 0; flex: 1; }
	.err { color: var(--danger); font-size: 13px; padding: 4px 12px; }
	@media (max-width: 1100px) { .grid { grid-template-columns: 1fr; } }
</style>
