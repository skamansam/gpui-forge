<script lang="ts">
	import type { Diagnostic } from './Canvas.svelte';
	let { diagnostics }: { diagnostics: Diagnostic[] } = $props();
</script>

<ul class:ok={diagnostics.length === 0}>
	{#each diagnostics as d, i (i)}
		<li><code>{d.line}:{d.col}</code> {d.message}{#if d.hint}<span class="hint"> — {d.hint}</span>{/if}</li>
	{:else}
		<li>No problems: the document is valid against the gpui-kit catalog.</li>
	{/each}
</ul>

<style>
	ul { list-style: none; margin: 0; padding: 8px 12px; font-size: 13px; color: var(--danger); max-height: 140px; overflow: auto; }
	ul.ok { color: #4ade80; }
	.hint { color: var(--muted); }
</style>
