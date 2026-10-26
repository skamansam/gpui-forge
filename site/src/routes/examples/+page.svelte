<script lang="ts">
	import { resolve } from '$app/paths';
	import Canvas from '#lib/Canvas.svelte';
	import YamlEditor from '#lib/YamlEditor.svelte';
	import { examples } from '#lib/examples.ts';

	let tab = $state<Record<string, 'yaml' | 'rust'>>({});
	const share = (yaml: string) => `${resolve('/playground')}#${encode(yaml)}`;
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
			<p>{ex.summary} <a href={share(ex.yaml)}>Open in the playground →</a></p>
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
</div>

<style>
	.page { max-width: 1200px; margin: 0 auto; padding: 32px 24px 80px; }
	.lead { color: var(--muted); }
	section { margin-top: 40px; }
	.grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
	.panel-title button { background: none; border: 0; color: var(--muted); font: inherit; text-transform: inherit; letter-spacing: inherit; cursor: pointer; padding: 0 8px 0 0; }
	.panel-title button.on { color: var(--fg); }
	pre.rust { margin: 0; border: 0; border-radius: 0; height: 420px; }
	@media (max-width: 900px) { .grid { grid-template-columns: 1fr; } }
</style>
