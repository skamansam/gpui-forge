<script lang="ts">
	let { data } = $props();
</script>

<svelte:head><title>{data.doc.title} — GPUI Forge</title></svelte:head>

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

<style>
	article :global(h1) { margin-top: 0; }
	article :global(h2[id]),
	article :global(h3[id]) { scroll-margin-top: 80px; }
	.note { margin-top: 48px; padding-top: 16px; border-top: 1px solid var(--border); color: var(--muted); font-size: 13px; line-height: 1.6; font-style: italic; }
</style>
