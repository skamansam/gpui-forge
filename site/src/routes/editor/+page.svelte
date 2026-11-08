<script lang="ts">
	import { parse as parseYaml, isMap } from 'yaml';
	import Canvas, { type Diagnostic } from '#lib/Canvas.svelte';
	import YamlEditor from '#lib/YamlEditor.svelte';
	import Diagnostics from '#lib/Diagnostics.svelte';
	import { examples } from '#lib/examples.ts';
	import * as F from '#lib/forge-doc.ts';

	type Prop = { type?: string; values?: string[]; default?: unknown; summary?: string } | string;
	type Element = { summary?: string; content?: string; props?: Record<string, Prop>; children?: unknown };
	type Base = { id: string; name?: string; catalog?: string; wasm?: boolean; libraries?: Record<string, unknown> };

	// The shipped registry — every `bases/*.base.yaml` manifest and every
	// `catalogs/*.catalog.yaml`, keyed by library id (#98 base picker).
	const baseFiles = import.meta.glob('../../../../bases/*.base.yaml', {
		query: '?raw', import: 'default', eager: true,
	});
	const catalogFiles = import.meta.glob('../../../../catalogs/*.catalog.yaml', {
		query: '?raw', import: 'default', eager: true,
	});
	const bases: Base[] = Object.values(baseFiles).map((s) => parseYaml(s as string));
	const catalogs: Record<string, Record<string, Element>> = Object.fromEntries(
		Object.values(catalogFiles).map((s) => {
			const m = parseYaml(s as string);
			return [m.library, m.elements];
		}),
	);

	let yaml = $state(examples[0].yaml);
	let selected = $state<number[]>([]);
	let diagnostics = $state<Diagnostic[]>([]);
	let hooks = $state<string[]>([]);
	let undoStack = $state<string[]>([]);
	let redoStack = $state<string[]>([]);
	let fileInput: HTMLInputElement;

	const STORE = 'forge-editor-doc';
	if (typeof localStorage !== 'undefined' && localStorage.getItem(STORE)) {
		yaml = localStorage.getItem(STORE)!;
	}
	$effect(() => {
		localStorage.setItem(STORE, yaml);
	});

	const doc = $derived(F.parse(yaml));
	const root = $derived(doc ? F.tree(doc) : null);
	const node = $derived(doc ? F.nodeAt(doc, selected) : null);
	const key = $derived(node ? F.elementKey(node) : undefined);

	// The doc's declared base (first registered when undeclared) drives the
	// palette: its catalog plus every `use:`-imported library the base marks
	// compatible. `forge.*` intrinsics stay out of the palette.
	const docBase = $derived(String(doc?.get('base') ?? ''));
	const baseId = $derived(bases.some((b) => b.id === docBase) ? docBase : (bases[0]?.id ?? 'gpui-kit'));
	const baseDef = $derived(bases.find((b) => b.id === baseId));
	const catalog = $derived(catalogs[baseDef?.catalog ?? 'gpui-kit'] ?? {});
	const libs = $derived.by(() => {
		const out: { id: string; tag: string; elements: Record<string, Element> }[] = [
			{ id: baseId, tag: '', elements: catalog },
		];
		const use = doc?.get('use');
		const compatible = new Set(Object.keys(baseDef?.libraries ?? {}));
		if (isMap(use)) {
			for (const it of use.items) {
				const alias = String((it.key as { value?: string })?.value ?? it.key);
				const lib = String((it.value as { value?: string })?.value ?? it.value);
				if (lib === 'forge' || !compatible.has(lib) || !catalogs[lib]) continue;
				out.push({ id: `${alias} (${lib})`, tag: `${alias}.`, elements: catalogs[lib] });
			}
		}
		return out;
	});

	const def = $derived(key ? catalog[F.elementName(key)] : undefined);
	const canHaveChildren = $derived(!!def?.children);

	const propType = (p: Prop) => (typeof p === 'string' ? p : (p.type ?? 'string'));
	const same = (a: number[], b: number[]) => a.length === b.length && a.every((x, i) => x === b[i]);

	/** Applies a structural edit to a fresh parse of the text and writes the text back. */
	function edit(change: (d: NonNullable<typeof doc>) => void) {
		const d = F.parse(yaml);
		if (!d) return;
		const before = yaml;
		change(d);
		const after = F.stringify(d);
		if (after === before) return;
		undoStack = [...undoStack.slice(-99), before];
		redoStack = [];
		yaml = after;
	}
	function undo() { if (undoStack.length) { redoStack = [...redoStack, yaml]; yaml = undoStack.pop()!; } }
	function redo() { if (redoStack.length) { undoStack = [...undoStack, yaml]; yaml = redoStack.pop()!; } }

	function onKey(e: KeyboardEvent) {
		if ((e.target as HTMLElement)?.tagName.match(/INPUT|TEXTAREA|SELECT|CODEMIRROR/)) return;
		if (e.key === 'Delete' && selected.length) { e.preventDefault(); edit((d) => F.removeNode(d, selected)); selected = selected.slice(0, -1); }
		if (!e.ctrlKey && !e.metaKey) return;
		if (e.key === 'z' && !e.shiftKey) { e.preventDefault(); undo(); }
		if (e.key === 'y' || (e.key === 'z' && e.shiftKey)) { e.preventDefault(); redo(); }
		if (e.key === 's') { e.preventDefault(); download(); }
	}

	const stamp = () => new Date().toISOString().slice(0, 10);
	function downloadText(name: string, text: string, type: string) {
		const a = document.createElement('a');
		a.href = URL.createObjectURL(new Blob([text], { type }));
		a.download = name;
		a.click();
		URL.revokeObjectURL(a.href);
	}
	function download() { downloadText(`doc-${stamp()}.forge.yaml`, yaml, 'text/yaml'); }
	function exportJson() {
		const j = F.toJson(yaml);
		if (j) downloadText(`doc-${stamp()}.json`, j, 'application/json');
	}
	let copied = $state(false);
	const encode = (s: string) => btoa(String.fromCharCode(...new TextEncoder().encode(s)));
	function loadExample(i: number) {
		if (i < 0) return;
		yaml = examples[i].yaml;
	}
	async function share() {
		history.replaceState(null, '', `#${encode(JSON.stringify({ yaml }))}`);
		await navigator.clipboard?.writeText(location.href);
		copied = true;
		setTimeout(() => (copied = false), 1500);
	}
	async function openFile(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file) return;
		const text = await file.text();
		if (file.name.endsWith('.json')) {
			const d = F.fromJson(text);
			if (d) { yaml = F.stringify(d); } else { alert('Not a forge JSON document'); }
		} else {
			yaml = text;
		}
		(e.target as HTMLInputElement).value = '';
	}

	function dropAt(path: number[], e: DragEvent) {
		const name = e.dataTransfer?.getData('forge-element');
		if (!name) return;
		e.preventDefault();
		// Drop on a container inserts inside; on a leaf it lands next to it.
		const m = F.nodeAt(doc!, path);
		const el = m && catalog[F.elementName(F.elementKey(m) ?? '') ?? ''];
		const target = el?.children ? path : path.slice(0, -1);
		if (target.length) edit((d) => F.addChild(d, target, name));
		selected = target;
	}
	const at = (d: NonNullable<typeof doc>) => F.nodeAt(d, selected)!;
</script>

<svelte:head><title>Editor — GPUI Forge</title></svelte:head>
<svelte:window onkeydown={onKey} />

{#snippet branch(n: F.TreeNode)}
	<li>
		<button class="node" class:sel={same(n.path, selected)} onclick={() => (selected = n.path)}
		ondragover={(e) => e.preventDefault()} ondrop={(e) => dropAt(n.path, e)}>{n.key}</button>
		{#if n.children.length}<ul>{#each n.children as c (c.path.join('.'))}{@render branch(c)}{/each}</ul>{/if}
	</li>
{/snippet}

<div class="editor">
	<div class="panel col palette">
		<div class="panel-title"><span>Components</span>
			<select value={baseId} title="base component system"
				onchange={(e) => edit((d) => d.set('base', e.currentTarget.value))}>
				{#each bases as b (b.id)}<option value={b.id}>{b.name ?? b.id}</option>{/each}
			</select>
		</div>
		<p class="hint">{canHaveChildren ? `Click to add inside ${key}` : 'Select a container to add children'}</p>
		<div class="list">
			{#each libs as lib (lib.id)}
				{#if lib.tag}<div class="lib">{lib.id}</div>{/if}
				{#each Object.entries(lib.elements) as [name, el] (lib.tag + name)}
					<button class="item" disabled={!canHaveChildren} title={el.summary} draggable="true"
						ondragstart={(e) => e.dataTransfer?.setData('forge-element', lib.tag + name)}
						onclick={() => edit((d) => F.addChild(d, selected, lib.tag + name))}>{lib.tag}{name}</button>
				{/each}
			{/each}
		</div>
	</div>

	<div class="panel col">
		<div class="panel-title"><span>Tree</span>
			<span class="tools">
				<button disabled={!undoStack.length} onclick={undo} title="Undo (Ctrl+Z)">↩</button>
				<button disabled={!redoStack.length} onclick={redo} title="Redo (Ctrl+Y)">↪</button>
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
		<div class="panel-title"><span>Canvas — {baseId}</span><span>{hooks.at(-1) ? `hook: ${hooks.at(-1)}` : ''}</span></div>
		{#if baseDef?.wasm}
			<Canvas {yaml} base={baseId} bindings={examples[0].state} height="calc(100vh - 300px)" ondiagnostics={(d) => (diagnostics = d)} onhook={(c) => (hooks = [...hooks, c])} />
		{:else}
			<div class="desktop-only">Desktop preview only — the {baseDef?.name ?? baseId} base has no WASM canvas.</div>
		{/if}
		<Diagnostics {diagnostics} />
	</div>

	<div class="panel col">
		<div class="panel-title"><span>login.forge.yaml</span>
			<span class="tools">
				<select title="load an example" onchange={(e) => loadExample(+e.currentTarget.value)}>
					<option value="-1">Examples…</option>
					{#each examples as ex, i (ex.slug)}<option value={i}>{ex.title}</option>{/each}
				</select>
				<button onclick={() => fileInput.click()} title="Open .forge.yaml or .json">Open</button>
				<button onclick={download} title="Download .forge.yaml (Ctrl+S)">Save</button>
				<button onclick={exportJson} title="Download JSON model">JSON</button>
				<button onclick={share} title="Copy a share link">{copied ? 'Copied' : 'Share'}</button>
			</span>
		</div>
		<input bind:this={fileInput} type="file" accept=".yaml,.yml,.json,.forge.yaml" hidden onchange={openFile} />
		<YamlEditor bind:value={yaml} height="calc(100vh - 160px)" />
	</div>
</div>

<style>
	.editor { display: grid; grid-template-columns: 170px 280px 1fr 1fr; gap: 10px; padding: 12px; }
	.col { display: flex; flex-direction: column; min-width: 0; }
	.hint { color: var(--muted); font-size: 12px; margin: 8px 12px; }
	.desktop-only { flex: 1; display: grid; place-items: center; color: var(--muted); font-size: 13px; padding: 24px; text-align: center; }
	.list { display: flex; flex-direction: column; overflow: auto; }
	.lib { color: var(--muted); font-size: 11px; margin: 8px 12px 2px; text-transform: uppercase; letter-spacing: 0.05em; }
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
