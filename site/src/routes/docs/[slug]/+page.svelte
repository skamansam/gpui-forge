<script lang="ts">
	import { resolve } from '$app/paths';
	let { data } = $props();
</script>

<svelte:head><title>{data.doc.title} — GPUI Forge</title></svelte:head>

<div class="layout">
	<aside>
		<h4>Specification</h4>
		{#each data.docs as d (d.slug)}
			<a href={resolve('/docs/[slug]', { slug: d.slug })} class:active={d.slug === data.doc.slug}>{d.title}</a>
		{/each}
	</aside>
	<!-- The HTML is rendered at build time from docs/spec in this repository. -->
	<!-- eslint-disable-next-line svelte/no-at-html-tags -->
	<article>{@html data.doc.html}</article>
</div>

<style>
	.layout { display: grid; grid-template-columns: 220px minmax(0, 820px); gap: 40px; max-width: 1100px; margin: 0 auto; padding: 32px 24px 80px; }
	aside { position: sticky; top: 80px; align-self: start; display: flex; flex-direction: column; gap: 4px; }
	aside h4 { margin: 16px 0 4px; font-size: 12px; text-transform: uppercase; letter-spacing: 0.06em; color: var(--muted); }
	aside a { color: var(--muted); padding: 2px 0; }
	aside a.active { color: var(--fg); font-weight: 600; }
	article :global(h1) { margin-top: 0; }
	@media (max-width: 800px) { .layout { grid-template-columns: 1fr; } aside { position: static; } }
</style>
