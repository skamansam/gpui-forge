<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { categories } from '#lib/docs.ts';

	let { children } = $props();
	let drawer = $state(false);
	let active = $state('');

	const toc = $derived(page.data.doc?.toc ?? []);

	// Track the section under the viewport top for the right rail.
	$effect(() => {
		if (!toc.length) return;
		const heads = [...document.querySelectorAll('.content h2[id], .content h3[id]')];
		const onScroll = () => {
			let current = heads[0]?.id ?? '';
			for (const h of heads) {
				if (h.getBoundingClientRect().top < 96) current = h.id;
				else break;
			}
			active = current;
		};
		onScroll();
		addEventListener('scroll', onScroll, { passive: true });
		return () => removeEventListener('scroll', onScroll);
	});
</script>

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
							class:active={d.slug === page.data.doc?.slug}
							onclick={() => (drawer = false)}>{d.title}</a
						>
					{/each}
				</details>
			{/each}
		</nav>
	</aside>
	<div class="content">
		{@render children()}
	</div>
	{#if toc.length > 1}
		<nav class="toc" aria-label="On this page">
			<span class="toc-title">On this page</span>
			{#each toc as t (t.id)}
				<a href="#{t.id}" class:d3={t.depth === 3} class:active={active === t.id}>{t.text}</a>
			{/each}
		</nav>
	{/if}
</div>

<style>
	.layout { display: grid; grid-template-columns: 240px minmax(0, 820px) 180px; gap: 40px; max-width: 1350px; margin: 0 auto; padding: 32px 24px 80px; }
	.layout:not(:has(.toc)) { grid-template-columns: 240px minmax(0, 820px); }
	aside { position: sticky; top: 80px; align-self: start; max-height: calc(100vh - 100px); overflow-y: auto; }
	aside nav { display: flex; flex-direction: column; gap: 12px; }
	aside details { border-left: 1px solid var(--border); }
	aside summary { cursor: pointer; padding: 2px 0 4px 12px; font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--fg); list-style-position: inside; }
	aside summary::marker { color: var(--muted); }
	aside details a { display: block; color: var(--muted); padding: 3px 0 3px 24px; border-left: 1px solid transparent; margin-left: -1px; }
	aside details a:hover { color: var(--fg); }
	aside details a.active { color: var(--accent-2); border-left-color: var(--accent); }
	.toc { position: sticky; top: 80px; align-self: start; display: flex; flex-direction: column; gap: 2px; max-height: calc(100vh - 100px); overflow-y: auto; border-left: 1px solid var(--border); padding-left: 12px; }
	.toc-title { font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--fg); margin-bottom: 4px; }
	.toc a { color: var(--muted); font-size: 13px; padding: 2px 0; }
	.toc a.d3 { padding-left: 14px; }
	.toc a:hover { color: var(--fg); }
	.toc a.active { color: var(--accent-2); }
	.content { min-width: 0; }
	.drawer-toggle, .backdrop { display: none; }
	@media (max-width: 1150px) {
		.layout, .layout:not(:has(.toc)) { grid-template-columns: 240px minmax(0, 1fr); }
		.toc { display: none; }
	}
	@media (max-width: 800px) {
		.layout, .layout:not(:has(.toc)) { grid-template-columns: 1fr; }
		.drawer-toggle { display: block; position: sticky; top: 64px; z-index: 25; margin: -8px 0 8px; padding: 8px 14px; border: 1px solid var(--border); border-radius: 8px; background: var(--panel-2); color: var(--fg); font: inherit; cursor: pointer; width: fit-content; }
		.backdrop { display: block; position: fixed; inset: 0; z-index: 39; background: rgb(0 0 0 / 0.55); border: 0; padding: 0; cursor: default; }
		aside { position: fixed; inset: 0 auto 0 0; z-index: 40; width: min(300px, 85vw); max-height: none; padding: 24px 20px 24px 24px; background: var(--bg); border-right: 1px solid var(--border); transform: translateX(-100%); transition: transform 0.18s ease-out; }
		aside.open { transform: none; }
	}
</style>
