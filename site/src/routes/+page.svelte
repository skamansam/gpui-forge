<script lang="ts">
	import { resolve } from '$app/paths';
	import Canvas from '#lib/Canvas.svelte';
	import TuiCanvas from '#lib/TuiCanvas.svelte';
	import YamlEditor from '#lib/YamlEditor.svelte';
	import { examples, showcases } from '#lib/examples.ts';

	const example = examples[0];
	const tui = showcases.find((s) => s.slug === 'ratatui-dashboard')!;
	let yaml = $state(example.yaml);
	let hooks = $state<string[]>([]);

	const tools = [
		{
			name: 'Web editor',
			href: '/editor',
			doc: 'editor',
			blurb: 'Compose documents in the browser — palette, tree, inspector, live WASM canvas, share links. No install.'
		},
		{
			name: 'Forge Studio',
			href: '/docs/studio',
			doc: 'studio',
			blurb: 'The native workbench — drag-and-drop editing, host previews, themes, dialogs as real windows.'
		},
		{
			name: 'forge CLI',
			href: '/docs/forge-cli',
			doc: 'forge-cli',
			blurb: 'check · fmt · new · build · package — validate a document or compile it into a native app.'
		},
		{
			name: 'Preview hosts',
			href: '/docs/studio-preview',
			doc: 'studio-preview',
			blurb: 'forge-preview-<base> binaries run docs on their native event loop — Studio drives them over IPC.'
		}
	];

	const bases = ['gpui-kit', 'ratatui', 'iced', 'bezel', 'gpui02', 'adabraka'];
	const libs = ['ely', 'gpui-component', 'gpui-flow', 'base-gpui', 'video', 'apple-hig', 'pdf', 'whiteboard', 'ratex'];
</script>

<svelte:head><title>GPUI Forge — compose GPUI interfaces visually</title></svelte:head>

<section class="hero">
	<h1>Describe a GPUI view in YAML.<br /><span>See it rendered for real.</span></h1>
	<p>
		GPUI Forge keeps interfaces in plain, line-oriented YAML that any editor and diff tool
		understands, and renders them with real components — natively, or in the browser through
		WebAssembly. Everything below is the actual renderer, running now.
	</p>
	<div class="cta">
		<a class="btn primary" href={resolve('/editor')}>Open the editor</a>
		<a class="btn" href={resolve('/docs')}>Read the docs</a>
		<a class="btn" href="https://github.com/skamansam/gpui-forge">GitHub</a>
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

<section class="tui">
	<div class="tui-copy">
		<h2>Desktop apps <em>and</em> terminal apps</h2>
		<p>
			The same document format drives every base. Set <code>base: ratatui</code> and your
			layout renders on a real <code>TestBackend</code> — blocks, lists, gauges, tabs,
			inputs, tables — drawn here as a live terminal frame.
		</p>
		<a href={resolve('/examples')}>More examples →</a>
	</div>
	<div class="panel">
		<div class="panel-title"><span>ratatui-dashboard.forge.yaml — terminal frame</span></div>
		<TuiCanvas yaml={tui.yaml} height="300px" />
	</div>
</section>

<section class="features">
	<div>
		<h3>Plain YAML, one fact per line</h3>
		<p>Standard YAML 1.2 with a JSON Schema: YAML↔JSON converters, linters and schema-aware editors work unchanged, and diffs stay small.</p>
	</div>
	<div>
		<h3>Real components</h3>
		<p>Elements map one-to-one onto the base's components, and enum values use the Rust names (<code>Primary</code>, <code>Small</code>).</p>
	</div>
	<div>
		<h3>Checked against catalogs</h3>
		<p>Every library ships a catalog of its elements, props and events, so typos and wrong types are reported with line and column.</p>
	</div>
	<div>
		<h3>Native or web</h3>
		<p>The same document drives the native runtime, Forge Studio and this browser canvas, and later compiles to a standalone Cargo project.</p>
	</div>
	<div>
		<h3>Declarative control flow</h3>
		<p><code>if:</code>/<code>else:</code>, <code>for:</code>/<code>each:</code>/<code>key:</code>, <code>with:</code> and <code>match:</code>/<code>case</code> — UI behavior authored in the document, no backend injection.</p>
	</div>
	<div>
		<h3>Named refs, edited in place</h3>
		<p><code>&amp;name</code> blocks share structure via <code>&lt;&lt;: *name</code>; edits splice into the ref block so named refs stay intact across saves.</p>
	</div>
</section>

<section class="ecosystem">
	<h2>One document, four tools</h2>
	<div class="tools">
		{#each tools as t (t.name)}
			<a class="tool" href={resolve('/docs/[slug]', { slug: t.doc })}>
				<h3>{t.name}</h3>
				<p>{t.blurb}</p>
			</a>
		{/each}
	</div>
</section>

<section class="basesec">
	<h2>Bases &amp; libraries</h2>
	<p class="sub">
		A Rust app links exactly one GPUI — a <code>base:</code> picks which build and
		widget set a document renders on, and <code>use:</code> imports libraries
		compatible with it.
	</p>
	<div class="chips">
		<span class="chip-label">bases</span>
		{#each bases as b (b)}<span class="chip base">{b}</span>{/each}
	</div>
	<div class="chips">
		<span class="chip-label">use:</span>
		{#each libs as l (l)}<span class="chip">{l}</span>{/each}
	</div>
</section>

<section class="cta-bottom">
	<h2>Start building</h2>
	<div class="cta">
		<a class="btn primary" href={resolve('/docs/[slug]', { slug: 'getting-started' })}>Get started</a>
		<a class="btn" href={resolve('/examples')}>Browse the examples</a>
	</div>
</section>

<style>
	.hero { max-width: 900px; margin: 0 auto; padding: 72px 24px 32px; text-align: center; }
	h1 { font-size: 44px; margin: 0 0 16px; }
	h1 span { color: var(--accent); }
	.hero p { color: var(--muted); font-size: 17px; max-width: 720px; margin: 0 auto 24px; }
	.cta { display: flex; gap: 12px; justify-content: center; flex-wrap: wrap; }
	.live { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; max-width: 1200px; margin: 24px auto; padding: 0 24px; }
	.tui { display: grid; grid-template-columns: 340px 1fr; gap: 32px; align-items: center; max-width: 1200px; margin: 48px auto; padding: 0 24px; }
	.tui-copy h2 { margin: 0 0 10px; }
	.tui-copy p { color: var(--muted); margin: 0 0 14px; }
	.features { display: grid; grid-template-columns: repeat(4, 1fr); gap: 24px; max-width: 1200px; margin: 48px auto; padding: 0 24px; }
	.features h3 { margin: 0 0 6px; font-size: 16px; }
	.features p { margin: 0; color: var(--muted); font-size: 14px; }
	.ecosystem { max-width: 1200px; margin: 48px auto; padding: 0 24px; }
	.ecosystem h2, .basesec h2, .cta-bottom h2 { text-align: center; margin: 0 0 24px; }
	.tools { display: grid; grid-template-columns: repeat(4, 1fr); gap: 16px; }
	.tool { display: block; background: var(--panel); border: 1px solid var(--border); border-radius: 10px; padding: 18px; color: var(--fg); }
	.tool:hover { border-color: var(--accent); text-decoration: none; }
	.tool h3 { margin: 0 0 6px; font-size: 16px; }
	.tool p { margin: 0; color: var(--muted); font-size: 13px; line-height: 1.5; }
	.basesec { max-width: 900px; margin: 48px auto; padding: 0 24px; text-align: center; }
	.basesec .sub { color: var(--muted); max-width: 640px; margin: 0 auto 20px; }
	.chips { display: flex; gap: 8px; flex-wrap: wrap; justify-content: center; align-items: center; margin-bottom: 10px; }
	.chip-label { font-size: 12px; color: var(--muted); text-transform: uppercase; letter-spacing: 0.06em; margin-right: 4px; }
	.chip { font-family: var(--mono); font-size: 12px; padding: 4px 10px; border: 1px solid var(--border); border-radius: 999px; color: var(--muted); background: var(--panel); }
	.chip.base { color: var(--accent-2); border-color: var(--accent); }
	.cta-bottom { max-width: 700px; margin: 64px auto 96px; padding: 0 24px; text-align: center; }
	@media (max-width: 900px) { .live, .features, .tools, .tui { grid-template-columns: 1fr; } }
</style>
