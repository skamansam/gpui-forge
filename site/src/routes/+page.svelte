<script lang="ts">
	import { resolve } from '$app/paths';
	import Canvas from '#lib/Canvas.svelte';
	import YamlEditor from '#lib/YamlEditor.svelte';
	import { examples } from '#lib/examples.ts';

	const example = examples[0];
	let yaml = $state(example.yaml);
	let hooks = $state<string[]>([]);
</script>

<svelte:head><title>GPUI Forge — compose GPUI interfaces visually</title></svelte:head>

<section class="hero">
	<h1>Describe a GPUI view in YAML.<br /><span>See it rendered by gpui-kit.</span></h1>
	<p>
		GPUI Forge keeps interfaces in plain, line-oriented YAML that any editor and diff tool
		understands, and renders them with real GPUI components: natively, or in the browser through
		WebAssembly. Everything on this page is the actual gpui-kit renderer.
	</p>
	<div class="cta">
		<a class="btn primary" href={resolve('/playground')}>Open the playground</a>
		<a class="btn" href={resolve('/editor')}>Try the editor</a>
		<a class="btn" href={resolve('/docs')}>Read the spec</a>
	</div>
</section>

<section class="live">
	<div class="panel">
		<div class="panel-title"><span>login.forge.yaml</span><span>edit me</span></div>
		<YamlEditor bind:value={yaml} height="440px" />
	</div>
	<div class="panel">
		<div class="panel-title"><span>gpui-kit canvas (WASM)</span><span>{hooks.at(-1) ?? ''}</span></div>
		<Canvas {yaml} bindings={example.state} height="440px" onhook={(c) => (hooks = [...hooks, c])} />
	</div>
</section>

<section class="features">
	<div>
		<h3>Plain YAML, one fact per line</h3>
		<p>Standard YAML 1.2 with a JSON Schema: YAML↔JSON converters, linters and schema-aware editors work unchanged, and diffs stay small.</p>
	</div>
	<div>
		<h3>Real components</h3>
		<p>Elements map one-to-one onto gpui-kit components, and enum values use the Rust names (<code>Primary</code>, <code>Small</code>).</p>
	</div>
	<div>
		<h3>Checked against catalogs</h3>
		<p>Every library ships a catalog of its elements, props and events, so typos and wrong types are reported with line and column.</p>
	</div>
	<div>
		<h3>Native or web</h3>
		<p>The same document drives the native runtime, Forge Studio and this browser canvas, and later compiles to a standalone Cargo project.</p>
	</div>
</section>

<style>
	.hero { max-width: 900px; margin: 0 auto; padding: 72px 24px 32px; text-align: center; }
	h1 { font-size: 44px; margin: 0 0 16px; }
	h1 span { color: var(--accent); }
	.hero p { color: var(--muted); font-size: 17px; max-width: 720px; margin: 0 auto 24px; }
	.cta { display: flex; gap: 12px; justify-content: center; flex-wrap: wrap; }
	.live { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; max-width: 1200px; margin: 24px auto; padding: 0 24px; }
	.features { display: grid; grid-template-columns: repeat(4, 1fr); gap: 24px; max-width: 1200px; margin: 48px auto 80px; padding: 0 24px; }
	.features h3 { margin: 0 0 6px; font-size: 16px; }
	.features p { margin: 0; color: var(--muted); font-size: 14px; }
	@media (max-width: 900px) { .live, .features { grid-template-columns: 1fr; } }
</style>
