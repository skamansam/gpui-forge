<script lang="ts">
	import { parse as parseYaml } from 'yaml';
	import catalogSource from '../../../../catalogs/gpui-kit.catalog.yaml?raw';
	import Canvas, { type Diagnostic } from '#lib/Canvas.svelte';
	import YamlEditor from '#lib/YamlEditor.svelte';
	import Diagnostics from '#lib/Diagnostics.svelte';
	import { examples } from '#lib/examples.ts';
	import * as F from '#lib/forge-doc.ts';

	type Prop = { type?: string; values?: string[]; default?: unknown; summary?: string } | string;
	type Element = { summary?: string; content?: string; props?: Record<string, Prop>; children?: unknown };
	const catalog: Record<string, Element> = parseYaml(catalogSource).elements;

	let yaml = $state(examples[0].yaml);
	let selected = $state<number[]>([]);
	let diagnostics = $state<Diagnostic[]>([]);
	let hooks = $state<string[]>([]);

	const doc = $derived(F.parse(yaml));
	const root = $derived(doc ? F.tree(doc) : null);
	const node = $derived(doc ? F.nodeAt(doc, selected) : null);
	const key = $derived(node ? F.elementKey(node) : undefined);
	const def = $derived(key ? catalog[F.elementName(key)] : undefined);
	const canHaveChildren = $derived(!!def?.children);

	const propType = (p: Prop) => (typeof p === 'string' ? p : (p.type ?? 'string'));
	const same = (a: number[], b: number[]) => a.length === b.length && a.every((x, i) => x === b[i]);

	/** Applies a structural edit to a fresh parse of the text and writes the text back. */
	function edit(change: (d: NonNullable<typeof doc>) => void) {
		const d = F.parse(yaml);
		if (!d) return;
		change(d);
		yaml = F.stringify(d);
	}
	const at = (d: NonNullable<typeof doc>) => F.nodeAt(d, selected)!;
</script>

<svelte:head><title>Editor — GPUI Forge</title></svelte:head>

{#snippet branch(n: F.TreeNode)}
	<li>
		<button class="node" class:sel={same(n.path, selected)} onclick={() => (selected = n.path)}>{n.key}</button>
		{#if n.children.length}<ul>{#each n.children as c (c.path.join('.'))}{@render branch(c)}{/each}</ul>{/if}
	</li>
{/snippet}

<div class="editor">
	<div class="panel col palette">
		<div class="panel-title"><span>Components</span><span>gpui-kit</span></div>
		<p class="hint">{canHaveChildren ? `Click to add inside ${key}` : 'Select a container to add children'}</p>
		<div class="list">
			{#each Object.entries(catalog) as [name, el] (name)}
				<button class="item" disabled={!canHaveChildren} title={el.summary}
					onclick={() => edit((d) => F.addChild(d, selected, name))}>{name}</button>
			{/each}
		</div>
	</div>

	<div class="panel col">
		<div class="panel-title"><span>Tree</span>
			<span class="tools">
				<button disabled={!selected.length} onclick={() => edit((d) => { if (F.moveNode(d, selected, -1)) selected = [...selected.slice(0, -1), selected.at(-1)! - 1]; })}>↑</button>
				<button disabled={!selected.length} onclick={() => edit((d) => { if (F.moveNode(d, selected, 1)) selected = [...selected.slice(0, -1), selected.at(-1)! + 1]; })}>↓</button>
				<button disabled={!selected.length} onclick={() => { edit((d) => F.removeNode(d, selected)); selected = selected.slice(0, -1); }}>Delete</button>
			</span>
		</div>
		{#if root}<ul class="tree">{@render branch(root)}</ul>{:else}<p class="hint">Fix the YAML to see the tree.</p>{/if}
		<div class="panel-title"><span>Inspector</span><span>{key ?? ''}</span></div>
		<div class="inspector">
			{#if node && key && def}
				{#if def.summary}<p class="hint">{def.summary}</p>{/if}
				{#each Object.entries(def.props ?? {}) as [name, p] (name)}
					{@const type = propType(p)}
					{@const isContent = def.content === name}
					{@const field = isContent ? key : name}
					<label>
						<span>{name}{#if isContent}<em> (content)</em>{/if}</span>
						{#if type === 'bool'}
							<input type="checkbox" checked={F.getText(node, field) === 'true'}
								onchange={(e) => edit((d) => F.setBool(at(d), field, e.currentTarget.checked, false))} />
						{:else if type === 'enum' && typeof p !== 'string'}
							<select value={F.getText(node, field) || String(p.default ?? '')}
								onchange={(e) => edit((d) => F.setText(at(d), field, e.currentTarget.value === String(p.default) ? '' : e.currentTarget.value, isContent))}>
								{#each p.values ?? [] as v (v)}<option>{v}</option>{/each}
							</select>
						{:else}
							<input value={F.getText(node, field)} placeholder={type}
								onchange={(e) => edit((d) => F.setText(at(d), field, e.currentTarget.value, isContent))} />
						{/if}
					</label>
				{/each}
			{:else}
				<p class="hint">Select an element in the tree.</p>
			{/if}
		</div>
	</div>

	<div class="panel col">
		<div class="panel-title"><span>Canvas</span><span>{hooks.at(-1) ? `hook: ${hooks.at(-1)}` : ''}</span></div>
		<Canvas {yaml} bindings={examples[0].state} height="calc(100vh - 300px)" ondiagnostics={(d) => (diagnostics = d)} onhook={(c) => (hooks = [...hooks, c])} />
		<Diagnostics {diagnostics} />
	</div>

	<div class="panel col">
		<div class="panel-title"><span>login.forge.yaml</span><span>stays in sync</span></div>
		<YamlEditor bind:value={yaml} height="calc(100vh - 160px)" />
	</div>
</div>

<style>
	.editor { display: grid; grid-template-columns: 170px 280px 1fr 1fr; gap: 10px; padding: 12px; }
	.col { display: flex; flex-direction: column; min-width: 0; }
	.hint { color: var(--muted); font-size: 12px; margin: 8px 12px; }
	.list { display: flex; flex-direction: column; overflow: auto; }
	.item { text-align: left; background: none; border: 0; color: var(--fg); padding: 5px 12px; font: 13px var(--mono); cursor: pointer; }
	.item:hover:not(:disabled) { background: var(--panel-2); color: var(--accent-2); }
	.item:disabled { color: #52525b; cursor: default; }
	.tools button { background: var(--panel-2); color: var(--fg); border: 1px solid var(--border); border-radius: 4px; font-size: 12px; cursor: pointer; }
	.tools button:disabled { opacity: 0.4; }
	ul { list-style: none; margin: 0; padding-left: 14px; }
	.tree { padding: 8px; max-height: 38vh; overflow: auto; }
	.node { background: none; border: 0; color: var(--muted); font: 13px var(--mono); cursor: pointer; padding: 2px 6px; border-radius: 4px; }
	.node.sel { background: var(--accent); color: #111; }
	.inspector { padding: 4px 12px 12px; overflow: auto; flex: 1; }
	label { display: grid; grid-template-columns: 100px 1fr; align-items: center; gap: 8px; margin: 6px 0; font-size: 13px; }
	label em { color: var(--muted); font-style: normal; font-size: 11px; }
	input:not([type='checkbox']), select { background: var(--panel-2); color: var(--fg); border: 1px solid var(--border); border-radius: 6px; padding: 4px 6px; font: 13px var(--mono); min-width: 0; }
	input[type='checkbox'] { justify-self: start; }
	@media (max-width: 1200px) { .editor { grid-template-columns: 1fr 1fr; } }
</style>
