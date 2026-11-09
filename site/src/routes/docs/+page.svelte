<script lang="ts">
	import { resolve } from '$app/paths';
	import { categories } from '#lib/docs.ts';
</script>

<svelte:head><title>Docs — GPUI Forge</title></svelte:head>

<div class="index">
	<h1>Documentation</h1>
	<p class="lede">
		GPUI Forge is a declarative layer over GPUI: one <code>.forge.yaml</code> document drives the
		web editor, Forge Studio, the preview hosts, and the compiler. Pick a section —
		or start with <a href={resolve('/docs/[slug]', { slug: 'getting-started' })}>Getting started</a>.
	</p>
	{#each categories as cat (cat.id)}
		<section>
			<h2><a href={resolve('/docs/[slug]', { slug: cat.docs[0].slug })}>{cat.title}</a></h2>
			<p class="blurb">{cat.blurb}</p>
			<ul>
				{#each cat.docs as d (d.slug)}
					<li>
						<a href={resolve('/docs/[slug]', { slug: d.slug })}>{d.title}</a>
						<span class="sum">— {d.blurb}</span>
					</li>
				{/each}
			</ul>
		</section>
	{/each}
	<footer class="note">
		While I have been doing software development since the 1990s, I lack the time necessary to put
		forth my ideas into reality, so I have enlisted the help of
		<a href="https://devin.ai">Devin</a> and vibecoded this app into existence. If you notice
		anything wrong or see any bugs, please use the
		<a href="https://github.com/skamansam/gpui-forge/issues">gpui-forge GitHub issue reporting</a>
		to do so and I will attend to it as soon as I can. Thank you — I hope this app helps you in some
		small way.
	</footer>
</div>

<style>
	.index { max-width: 780px; margin: 0 auto; padding: 32px 24px 80px; }
	h1 { margin-bottom: 8px; }
	.lede { color: var(--muted); margin-bottom: 32px; }
	section { margin-bottom: 28px; }
	section h2 { font-size: 15px; text-transform: uppercase; letter-spacing: 0.06em; margin-bottom: 4px; }
	section h2 a { color: var(--fg); }
	.blurb { color: var(--muted); margin: 0 0 10px; }
	ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
	li { line-height: 1.45; }
	li a { font-weight: 500; }
	.sum { color: var(--muted); font-size: 13px; }
	.note { margin-top: 40px; padding-top: 16px; border-top: 1px solid var(--border); color: var(--muted); font-size: 13px; line-height: 1.6; font-style: italic; }
</style>
