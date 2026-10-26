// Edits Forge YAML structurally (the editor page) while keeping the text the source of truth.
import { parseDocument, isMap, isSeq, isScalar, YAMLMap, YAMLSeq, Scalar, type Document } from 'yaml';

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
