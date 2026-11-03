// Edits Forge YAML structurally (the editor page) while keeping the text the source of truth.
import { parseDocument, isMap, isSeq, isScalar, YAMLMap, YAMLSeq, Scalar, Document } from 'yaml';
import type { Document as ForgeDocument, Node as ForgeNode, Value as ForgeValue } from './forge-model';

export type TreeNode = { path: number[]; key: string; element: string; children: TreeNode[] };

const RESERVED = new Set(['style', 'children']);
export const isElementKey = (k: string) => /^[A-Za-z_][\w]*\.?[A-Z]/.test(k) || /^[A-Z]/.test(k);

export function elementKey(map: YAMLMap): string | undefined {
	for (const pair of map.items) {
		const k = String(isScalar(pair.key) ? pair.key.value : pair.key);
		if (!RESERVED.has(k) && !k.startsWith('on ') && isElementKey(k)) return k;
	}
}

export const elementName = (key: string) => key.split(/[#\s]/)[0].split('.').pop()!;

export function parse(text: string): Document | null {
	const doc = parseDocument(text);
	return doc.errors.length ? null : doc;
}

export function tree(doc: Document): TreeNode | null {
	const root = doc.get('root');
	return isMap(root) ? walk(root, []) : null;
}

function walk(map: YAMLMap, path: number[]): TreeNode {
	const key = elementKey(map) ?? '?';
	const kids = map.get('children');
	const children = isSeq(kids)
		? kids.items.flatMap((c, i) => (isMap(c) ? [walk(c, [...path, i])] : []))
		: [];
	return { path, key, element: elementName(key), children };
}

export function nodeAt(doc: Document, path: number[]): YAMLMap | null {
	let node = doc.get('root');
	for (const i of path) {
		const kids = isMap(node) ? node.get('children') : null;
		node = isSeq(kids) ? kids.items[i] : null;
	}
	return isMap(node) ? node : null;
}

export function getText(map: YAMLMap, key: string): string {
	const v = map.get(key, true);
	if (v == null) return '';
	if (isScalar(v)) return v.value == null ? '' : String(v.value);
	return String(v);
}

/** Sets a prop (or the content when `key` is the element key); empty removes a prop. */
export function setText(map: YAMLMap, key: string, value: string, isContent = false) {
	if (value === '' && !isContent) return void map.delete(key);
	const scalar = new Scalar(value === '' ? null : value);
	// Colors must be double-quoted, or YAML reads them as comments.
	if (value.startsWith('#')) scalar.type = Scalar.QUOTE_DOUBLE;
	map.set(key, scalar);
}

export function setBool(map: YAMLMap, key: string, value: boolean, fallback: boolean) {
	if (value === fallback) map.delete(key);
	else map.set(key, value);
}

export function addChild(doc: Document, path: number[], element: string) {
	const parent = nodeAt(doc, path);
	if (!parent) return;
	let kids = parent.get('children');
	if (!isSeq(kids)) {
		kids = new YAMLSeq();
		parent.set('children', kids);
	}
	// Parsed rather than built, so the empty value is written as `Element:` and not `? Element`.
	(kids as YAMLSeq).items.push(parseDocument(`${element}:`).contents);
}

export function removeNode(doc: Document, path: number[]) {
	if (!path.length) return;
	const parent = nodeAt(doc, path.slice(0, -1));
	const kids = parent?.get('children');
	if (isSeq(kids)) kids.items.splice(path.at(-1)!, 1);
}

export function moveNode(doc: Document, path: number[], by: -1 | 1) {
	const parent = nodeAt(doc, path.slice(0, -1));
	const kids = parent?.get('children');
	const i = path.at(-1)!;
	if (!isSeq(kids) || i + by < 0 || i + by >= kids.items.length) return false;
	[kids.items[i], kids.items[i + by]] = [kids.items[i + by], kids.items[i]];
	return true;
}

export const stringify = (doc: Document) => doc.toString({ nullStr: '', lineWidth: 0 });

// ── JSON model <-> YAML document (#38 import/export) ──────────────────────
// The JSON side is forge-spec's serde Document: props/style are [key, Value]
// pairs, events are {event, handler:{name,args}} and values are externally
// tagged enum objects like {"String":"x"}.

const pathFrom = (v: unknown[]): string =>
	v.map((s: any) => (s.Field !== undefined ? `.${s.Field}` : `[${s.Index}]`)).join('').replace(/^\./, '');

const UNIT: Record<string, string> = { Px: 'px', Rem: 'rem', Percent: '%', Fr: 'fr' };

/** A JSON-model value -> the scalar a Forge YAML map would hold. */
function fromJsonValue(v: unknown): unknown {
	if (!v || typeof v !== 'object' || Array.isArray(v)) return v;
	const [tag, inner] = Object.entries(v as Record<string, unknown>)[0] ?? [];
	switch (tag) {
		case 'String':
		case 'Enum':
			return inner;
		case 'Bool':
			return inner;
		case 'Number': {
			const n = inner as { value: number; unit?: string | null };
			return n.unit ? `${n.value}${UNIT[n.unit] ?? ''}` : n.value;
		}
		case 'Color': {
			const c = inner as { r: number; g: number; b: number; a: number };
			const hex = (x: number) => x.toString(16).padStart(2, '0');
			return `#${hex(c.r)}${hex(c.g)}${hex(c.b)}${c.a !== 255 ? hex(c.a) : ''}`;
		}
		case 'Token':
			return `$theme.${pathFrom(inner as unknown[])}`;
		case 'Binding': {
			const b = inner as { path: unknown[]; mode: string };
			return `{{ ${pathFrom(b.path)} }}${b.mode === 'TwoWay' ? ' <->' : ''}`;
		}
		case 'Interpolated':
			return (inner as any[])
				.map((s) => (s.Binding ? `{{ ${pathFrom(s.Binding)} }}` : s.Text))
				.join('');
		case 'List':
			return (inner as unknown[]).map(fromJsonValue);
		case 'Map':
			return Object.fromEntries((inner as [string, unknown][]).map(([k, x]) => [k, fromJsonValue(x)]));
		default:
			return v;
	}
}

function jsonToNode(n: any): Record<string, unknown> {
	const key = `${n.element?.namespace ? n.element.namespace + '.' : ''}${n.element?.name ?? '?'}` +
		(n.id ? `#${n.id}` : '');
	const map: Record<string, unknown> = { [key]: n.content != null ? fromJsonValue(n.content) : null };
	for (const [k, v] of n.props ?? []) map[k] = fromJsonValue(v);
	if (n.style?.length) map.style = Object.fromEntries(n.style.map(([k, v]: [string, unknown]) => [k, fromJsonValue(v)]));
	for (const e of n.events ?? []) map[`on ${e.event}`] = `${e.handler.name}(${(e.handler.args ?? []).map(fromJsonValue).join(', ')})`;
	if (n.children?.length) map.children = n.children.map(jsonToNode);
	return map;
}

/** Builds a Forge YAML `Document` (yaml AST) from a forge-spec JSON document. */
export function fromJson(text: string): Document | null {
	let m: ForgeDocument;
	try { m = JSON.parse(text); } catch { return null; }
	if (!m?.root) return null;
	const doc = new Document();
	doc.set('forge', String(m.version ?? '0.1'));
	if (m.base) doc.set('base', m.base);
	doc.set('root', doc.createNode(jsonToNode(m.root)));
	return doc;
}

const scalarToValue = (s: Scalar | null | undefined): unknown => {
	const t = (s?.value as string) ?? '';
	if (t === 'true' || t === 'false') return { Bool: t === 'true' };
	const num = t.match(/^(-?\d+(?:\.\d+)?)(px|rem|%|fr)?$/);
	if (num) {
		const unit = { px: 'Px', rem: 'Rem', '%': 'Percent', fr: 'Fr' }[num[2] ?? ''];
		return { Number: { value: parseFloat(num[1]), unit: unit ?? null } };
	}
	if (t.startsWith('#')) {
		const h = t.slice(1);
		if ([3, 4, 6, 8].includes(h.length) && /^[0-9a-fA-F]+$/.test(h)) {
			const l = (i: number) => parseInt(h.slice(i, i + 2), 16);
			const sh = (i: number) => parseInt(h[i], 16) * 17;
			const [r, g, b, a] = h.length <= 4 ? [sh(0), sh(1), sh(2), h.length === 4 ? sh(3) : 255]
				: [l(0), l(2), l(4), h.length === 8 ? l(6) : 255];
			return { Color: { r, g, b, a } };
		}
	}
	return { String: t };
};

function nodeToJson(map: YAMLMap): Record<string, unknown> {
	let element = { namespace: null as string | null, name: '?' }, id: string | null = null, content: unknown = null;
	const props: [string, unknown][] = [], style: [string, unknown][] = [], events: unknown[] = [], children: unknown[] = [];
	for (const it of map.items as any[]) {
		const k = String(it.key?.value ?? it.key);
		if (isElementKey(k) && isScalar(it.value)) {
			const [full] = k.split(/\s/);
			const [nameId] = k.split('#');
			const name = full.split('#')[0];
			const ns = name.includes('.') ? name.split('.')[0] : null;
			element = { namespace: ns, name: name.split('.').pop()! };
			id = nameId !== k ? k.slice(nameId.length + 1) : (k.includes('#') ? k.split('#')[1] : null);
			if (it.value.value !== null && it.value.value !== undefined && it.value.value !== '')
				content = scalarToValue(it.value);
		} else if (k === 'style' && isMap(it.value)) {
			for (const s of it.value.items as any[]) style.push([String(s.key?.value ?? s.key), scalarToValue(s.value)]);
		} else if (k === 'children' && isSeq(it.value)) {
			for (const c of it.value.items as any[]) if (isMap(c)) children.push(nodeToJson(c));
		} else if (k.startsWith('on ')) {
			const call = String(it.value?.value ?? '');
			const m = call.match(/^(\w+)\s*\((.*)\)\s*$/) ?? [null, call, ''];
			events.push({ event: k.slice(3), handler: { name: m[1], args: m[2] ? m[2].split(',').map(a => ({ String: a.trim() })) : [] } });
		} else if (k !== 'id') {
			props.push([k, isScalar(it.value) ? scalarToValue(it.value) : isMap(it.value)
				? { Map: it.value.items.map((e: any) => [String(e.key?.value ?? e.key), scalarToValue(e.value)]) }
				: { List: (it.value.items ?? []).map((e: any) => scalarToValue(e)) }]);
		}
	}
	return { element, id, content, props, style, events, children };
}

/** forge-spec JSON (serde model) for the document's root, or null if unparseable. */
export function toJson(text: string): string | null {
	const doc = parse(text);
	if (!doc || !isMap(doc.contents)) return null;
	const get = (k: string) => doc.get(k);
	const root = doc.get('root');
	if (!isMap(root)) return null;
	return JSON.stringify({
		schema: get('$schema') ?? null,
		version: get('forge') ?? '0.1',
		base: get('base') ?? null,
		imports: [],
		meta: [],
		dialogs: [],
		root: nodeToJson(root),
	}, null, 2);
}
