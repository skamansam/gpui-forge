import { marked } from 'marked';
import { resolve } from '$app/paths';

// `docs/<category>/<slug>.md` — the directory is the sidebar category, the
// filename the page slug (unique across the whole tree, so URLs stay
// `/docs/<slug>` even as pages move between categories).
const files = import.meta.glob('../../../docs/**/*.md', {
	query: '?raw',
	import: 'default',
	eager: true
}) as Record<string, string>;

const REPO_DOCS = 'https://github.com/skamansam/gpui-forge/blob/main/docs/format/';

const CATEGORY_ORDER: [id: string, title: string, blurb: string][] = [
	['start', 'Getting started', 'Install the toolchain, open the tools, ship your first document.'],
	['format', 'The .forge.yaml format', 'The document format every tool shares — syntax, model, styles, catalogs, validation, adapters.'],
	['studio', 'Forge Studio', 'The native workbench — visual editing, host previews, themes, projects.'],
	['web', 'Web app', 'The browser editor and showcase — canvases, sharing, examples.'],
	['cli', 'Forge CLI', 'The `forge` command-line — scaffolding, validating, building, packaging.']
];

const CATEGORY_IDS = new Set(CATEGORY_ORDER.map(([id]) => id));

// Within a category, pages follow this reading order; unlisted slugs sort last.
const PAGE_ORDER: Record<string, string[]> = {
	start: ['getting-started'],
	format: [
		'forge-yaml',
		'document-model',
		'style-layout',
		'catalog-manifest',
		'yaml-validation',
		'state-inspector',
		'adapters'
	],
	studio: ['studio', 'studio-editing', 'studio-preview', 'studio-themes', 'studio-projects'],
	web: ['editor', 'preview', 'web-examples'],
	cli: ['forge-cli', 'forge-toml']
};

export type TocEntry = { depth: number; text: string; id: string };

export type Doc = {
	slug: string;
	title: string;
	html: string;
	category: string;
	/** First paragraph after the title — the landing page's blurb. */
	blurb: string;
	/** h2/h3 outline for the "on this page" rail. */
	toc: TocEntry[];
};

export const docs: Doc[] = Object.entries(files)
	// Only category directories publish — docs/notes/ and friends stay internal.
	.filter(([path]) => CATEGORY_IDS.has(path.split('/').at(-2)!))
	.map(([path, md]) => {
		const parts = path.split('/');
		const slug = parts.pop()!.replace(/\.md$/, '');
		const category = parts.pop()!;
		const title = md.match(/^# (.+)$/m)?.[1] ?? slug;
		// h2/h3 outline — heading text gets link/code markup stripped for display.
		const seen = new Map<string, number>();
		const toc = [...md.matchAll(/^(#{2,3})\s+(.+?)\s*$/gm)].map((m) => {
			const text = m[2]
				.replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
				.replace(/[`*_]/g, '');
			let id = text
				.toLowerCase()
				.replace(/[^\p{L}\p{N}]+/gu, '-')
				.replace(/^-|-$/g, '');
			const n = seen.get(id) ?? 0;
			seen.set(id, n + 1);
			if (n) id = `${id}-${n}`;
			return { depth: m[1].length, text, id };
		});
		let heading = 0;
		const html = (marked.parse(md, { async: false }) as string)
			.replace(/<h([23])>/g, (m) => {
				const t = toc[heading++];
				return t ? `<h${t.depth} id="${t.id}">` : m;
			})
			.replace(
			/href="(?:\.\/)?([a-z0-9-]+)\.md(#[^"]*)?"/g,
			(_, s, hash = '') => `href="${resolve('/docs/[slug]', { slug: s })}${hash}"`
		).replace(
			// Other relative links point at files in the repository.
			/href="((?:\.\.?\/)[^"]*)"/g,
			(_, href) => `href="${new URL(href, REPO_DOCS).href}"`
		);
		const blurb =
			md
				.split(/\n{2,}/)
				.map((p) => p.trim())
				.find((p) => p && !p.startsWith('#'))
				?.replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
				.replace(/[`*_]/g, '')
				.slice(0, 160) ?? '';
		return { slug, title, html, category, blurb, toc };
	})
	.sort((a, b) => order(a) - order(b));

export type Category = { id: string; title: string; blurb: string; docs: Doc[] };

export const categories: Category[] = CATEGORY_ORDER.map(([id, title, blurb]) => ({
	id,
	title,
	blurb,
	docs: docs.filter((d) => d.category === id)
})).filter((c) => c.docs.length);

function order(doc: Doc) {
	const cat = CATEGORY_ORDER.findIndex(([id]) => id === doc.category);
	const page = (PAGE_ORDER[doc.category] ?? []).indexOf(doc.slug);
	return cat * 100 + (page < 0 ? 99 : page);
}
