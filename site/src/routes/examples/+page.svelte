<script lang="ts">
	import { resolve } from '$app/paths';
	import Canvas from '#lib/Canvas.svelte';
	import TuiCanvas from '#lib/TuiCanvas.svelte';
	import YamlEditor from '#lib/YamlEditor.svelte';
	import { examples, showcases } from '#lib/examples.ts';

	let tab = $state<Record<string, 'yaml' | 'rust'>>({});
	const share = (yaml: string, state: unknown) =>
		`${resolve('/editor')}#${encode(JSON.stringify({ yaml, state }))}`;
	const encode = (s: string) => btoa(String.fromCharCode(...new TextEncoder().encode(s)));
</script>

<svelte:head><title>Examples — GPUI Forge</title></svelte:head>

<div class="page">
	<h1>Examples</h1>
	<p class="lead">
		Each example shows the Forge file, the view gpui-kit renders from it, and roughly what the same
		view looks like written by hand in Rust.
	</p>
	{#each examples as ex (ex.slug)}
		<section id={ex.slug}>
			<h2>{ex.title}</h2>
			<p>{ex.summary} <a href={share(ex.yaml, ex.state)}>Open in the editor →</a></p>
			<div class="grid">
				<div class="panel">
					<div class="panel-title">
						<span>
							<button class:on={(tab[ex.slug] ?? 'yaml') === 'yaml'} onclick={() => (tab[ex.slug] = 'yaml')}>{ex.slug}.forge.yaml</button>
							<button class:on={tab[ex.slug] === 'rust'} onclick={() => (tab[ex.slug] = 'rust')}>hand-written Rust</button>
						</span>
					</div>
					{#if tab[ex.slug] === 'rust'}
						<pre class="rust">{ex.rust}</pre>
					{:else}
						<YamlEditor value={ex.yaml} readonly height="420px" />
					{/if}
				</div>
				<div class="panel">
					<div class="panel-title"><span>rendered by gpui-kit</span></div>
					<Canvas yaml={ex.yaml} bindings={ex.state} height="420px" />
				</div>
			</div>
		</section>
	{/each}

	<h2 class="sets">Widget sets</h2>
	<p class="lead">
		Every add-on library Forge Studio ships, as the
		<code>examples/*.forge.yaml</code> demo its Welcome tab opens. They render on a native base —
		the canvas above only previews gpui-kit — so these show as source (the ratatui demo draws a
		terminal frame). Open them in Studio
		(<code>cargo xtask studio</code> → Gallery) to run them.
	</p>
	{#each showcases as s (s.slug)}
		<section id={s.slug}>
			<h3>{s.title}</h3>
			<p>{s.summary}</p>
			{#if s.base === 'ratatui'}
				<div class="grid">
					<div class="panel">
						<div class="panel-title"><span>{s.slug}.forge.yaml</span></div>
						<YamlEditor value={s.yaml} readonly height="360px" />
					</div>
					<div class="panel">
						<div class="panel-title"><span>terminal frame</span></div>
						<TuiCanvas yaml={s.yaml} height="360px" />
					</div>
				</div>
			{:else}
				<div class="panel">
					<div class="panel-title"><span>{s.slug}.forge.yaml</span></div>
					<YamlEditor value={s.yaml} readonly height="360px" />
				</div>
			{/if}
		</section>
	{/each}
</div>

<style>
	.page { max-width: 1200px; margin: 0 auto; padding: 32px 24px 80px; }
	.lead { color: var(--muted); }
	.sets { margin-top: 64px; }
	section { margin-top: 40px; }
	.grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
	.panel-title button { background: none; border: 0; color: var(--muted); font: inherit; text-transform: inherit; letter-spacing: inherit; cursor: pointer; padding: 0 8px 0 0; }
	.panel-title button.on { color: var(--fg); }
	pre.rust { margin: 0; border: 0; border-radius: 0; height: 420px; }
	@media (max-width: 900px) { .grid { grid-template-columns: 1fr; } }
</style>
