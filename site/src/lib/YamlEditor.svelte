<script lang="ts">
	import { onMount } from 'svelte';
	import { EditorView, basicSetup } from 'codemirror';
	import { EditorState } from '@codemirror/state';
	import { yaml as yamlLanguage } from '@codemirror/lang-yaml';
	import { oneDark } from '@codemirror/theme-one-dark';

	let {
		value = $bindable(),
		readonly = false,
		height = '360px'
	}: { value: string; readonly?: boolean; height?: string } = $props();

	let host: HTMLDivElement;
	let view: EditorView | undefined;

	onMount(() => {
		view = new EditorView({
			parent: host,
			state: EditorState.create({
				doc: value,
				extensions: [
					basicSetup,
					yamlLanguage(),
					oneDark,
					EditorState.readOnly.of(readonly),
					EditorView.theme({ '&': { height: '100%' }, '.cm-scroller': { fontFamily: 'var(--mono)' } }),
					EditorView.updateListener.of((u) => {
						if (u.docChanged) value = u.state.doc.toString();
					})
				]
			})
		});
		return () => view?.destroy();
	});

	// Push outside changes (e.g. from the visual editor) into CodeMirror.
	$effect(() => {
		if (view && value !== view.state.doc.toString()) {
			view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: value } });
		}
	});
</script>

<div class="editor" style:height bind:this={host}></div>

<style>
	.editor { overflow: hidden; }
	.editor :global(.cm-editor) { height: 100%; font-size: 13px; }
</style>
