<script lang="ts">
	type Asset = { name: string; browser_download_url: string; size: number };
	type Release = { tag_name: string; name: string; body: string; assets: Asset[]; published_at: string };
	let releases = $state<Release[]>([]);
	let error = $state('');

	if (typeof fetch !== 'undefined') {
		fetch('https://api.github.com/repos/skamansam/gpui-forge/releases?per_page=10')
			.then((r) => (r.ok ? r.json() : Promise.reject(r.status)))
			.then((r) => (releases = r))
			.catch(() => (error = 'Could not load releases from GitHub.'));
	}
	const mb = (n: number) => `${(n / 1e6).toFixed(1)} MB`;
	const os = (name: string) =>
		name.includes('linux') ? 'Linux' : name.includes('macos') ? 'macOS' : name.includes('windows') ? 'Windows' : name;
</script>

<svelte:head><title>Releases — GPUI Forge</title></svelte:head>

<main>
	<h1>Releases</h1>
	<p class="hint">The <code>forge</code> CLI, prebuilt per platform. Tag a commit <code>v*</code> to cut a release.</p>
	{#if error}<p class="err">{error}</p>{/if}
	{#each releases as r (r.tag_name)}
		<section class="rel">
			<h2>{r.name || r.tag_name} <span class="hint">{r.published_at.slice(0, 10)}</span></h2>
			{#if r.assets.length}
				<ul>
					{#each r.assets as a (a.name)}
						<li><a href={a.browser_download_url}>{os(a.name)} — {a.name}</a> <span class="hint">{mb(a.size)}</span></li>
					{/each}
				</ul>
			{:else}
				<p class="hint">No binaries yet.</p>
			{/if}
		</section>
	{:else}
		{#if !error}<p class="hint">Loading…</p>{/if}
	{/each}
</main>

<style>
	main { max-width: 720px; margin: 0 auto; padding: 32px 16px; }
	.rel { border-top: 1px solid var(--border); padding: 12px 0; }
	.hint { color: var(--muted); font-size: 13px; }
	.err { color: var(--err, #f87171); }
	ul { list-style: none; padding-left: 0; }
	a { color: var(--accent-2); }
</style>
