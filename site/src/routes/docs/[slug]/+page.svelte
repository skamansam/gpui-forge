<script lang="ts">
	import { resolve } from '$app/paths';
	import { categories } from '#lib/docs.ts';
	let { data } = $props();
	let drawer = $state(false);
</script>

<svelte:head><title>{data.doc.title} — GPUI Forge</title></svelte:head>

<div class="layout">
	<button class="drawer-toggle" onclick={() => (drawer = true)}>☰ Documentation</button>
	{#if drawer}
		<button class="backdrop" aria-label="Close menu" onclick={() => (drawer = false)}></button>
	{/if}
	<aside class:open={drawer}>
		<nav>
			{#each categories as cat (cat.id)}
				<details open>
					<summary>{cat.title}</summary>
					{#each cat.docs as d (d.slug)}
						<a
							href={resolve('/docs/[slug]', { slug: d.slug })}
							class:active={d.slug === data.doc.slug}
							onclick={() => (drawer = false)}>{d.title}</a
						>
					{/each}
				</details>
			{/each}
		</nav>
	</aside>
	<!-- The HTML is rendered at build time from docs/ in this repository. -->
	<!-- eslint-disable-next-line svelte/no-at-html-tags -->
	<article>
		{@html data.doc.html}
		<footer class="note">
			While I have been doing software development since the 1990s, I lack the time necessary to
			put forth my ideas into reality, so I have enlisted the help of
			<a href="https://devin.ai">Devin</a> and vibecoded this app into existence. If you notice
			anything wrong or see any bugs, please use the
			<a href="https://github.com/skamansam/gpui-forge/issues">gpui-forge GitHub issue reporting</a>
			to do so and I will attend to it as soon as I can. Thank you — I hope this app helps you in
			some small way.
		</footer>
	</article>
</div>

<style>
	.layout { display: grid; grid-template-columns: 240px minmax(0, 820px); gap: 40px; max-width: 1150px; margin: 0 auto; padding: 32px 24px 80px; }
	aside { position: sticky; top: 80px; align-self: start; max-height: calc(100vh - 100px); overflow-y: auto; }
	aside nav { display: flex; flex-direction: column; gap: 12px; }
	aside details { border-left: 1px solid var(--border); }
	aside summary { cursor: pointer; padding: 2px 0 4px 12px; font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--fg); list-style-position: inside; }
	aside summary::marker { color: var(--muted); }
	aside details a { display: block; color: var(--muted); padding: 3px 0 3px 24px; border-left: 1px solid transparent; margin-left: -1px; }
	aside details a:hover { color: var(--fg); }
	aside details a.active { color: var(--accent-2); border-left-color: var(--accent); }
	.drawer-toggle, .backdrop { display: none; }
	article :global(h1) { margin-top: 0; }
	.note { margin-top: 48px; padding-top: 16px; border-top: 1px solid var(--border); color: var(--muted); font-size: 13px; line-height: 1.6; font-style: italic; }
	@media (max-width: 800px) {
		.layout { grid-template-columns: 1fr; }
		.drawer-toggle { display: block; position: sticky; top: 64px; z-index: 25; margin: -8px 0 8px; padding: 8px 14px; border: 1px solid var(--border); border-radius: 8px; background: var(--panel-2); color: var(--fg); font: inherit; cursor: pointer; width: fit-content; }
		.backdrop { display: block; position: fixed; inset: 0; z-index: 39; background: rgb(0 0 0 / 0.55); border: 0; padding: 0; cursor: default; }
		aside { position: fixed; inset: 0 auto 0 0; z-index: 40; width: min(300px, 85vw); max-height: none; padding: 24px 20px 24px 24px; background: var(--bg); border-right: 1px solid var(--border); transform: translateX(-100%); transition: transform 0.18s ease-out; }
		aside.open { transform: none; }
	}
</style>
