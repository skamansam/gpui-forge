<script lang="ts">
	import '../app.css';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';

	let { children } = $props();
	const links = [
		{ href: resolve('/docs'), label: 'Docs' },
		{ href: resolve('/examples'), label: 'Examples' },
		{ href: resolve('/playground'), label: 'Playground' },
		{ href: resolve('/editor'), label: 'Editor' }
	];
</script>

<header>
	<a class="brand" href={resolve('/')}><span class="mark">◆</span> GPUI Forge</a>
	<nav>
		{#each links as link (link.href)}
			<a href={link.href} class:active={page.url.pathname.startsWith(link.href)}>{link.label}</a>
		{/each}
		<a href="https://github.com/skamansam/gpui-forge">GitHub</a>
	</nav>
</header>

<main>{@render children()}</main>

<style>
	header { position: sticky; top: 0; z-index: 10; display: flex; align-items: center; justify-content: space-between; padding: 12px 24px; border-bottom: 1px solid var(--border); background: rgb(10 10 10 / 0.85); backdrop-filter: blur(8px); }
	.brand { color: var(--fg); font-weight: 700; font-size: 17px; }
	.mark { color: var(--accent); }
	nav { display: flex; gap: 20px; }
	nav a { color: var(--muted); }
	nav a.active, nav a:hover { color: var(--fg); text-decoration: none; }
	main { min-height: calc(100vh - 56px); }
</style>
