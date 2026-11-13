// Generates static/llms.txt and static/llms-full.txt from the real doc sources
// in ../docs. Runs via `prebuild`/`predev` so the files stay in sync with the
// docs tree — edit the docs, rebuild, and both files update. Both outputs are
// gitignored; llms.txt is a generated artifact, not a hand-maintained file.
//
// Section layout and the short descriptions of the app routes (editor,
// examples, releases) are curated here. Doc-page descriptions are pulled from
// each page's first paragraph, the same blurb the docs index shows.

import { execSync } from 'node:child_process';
import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';

const SITE = fileURLToPath(new URL('..', import.meta.url)); // site/
const REPO = fileURLToPath(new URL('../..', import.meta.url)); // repo root

// Custom domain comes from static/CNAME (same file GitHub Pages reads). The
// BASE_PATH env mirrors vite.config.ts for project-pages deploys.
const cname = readFileSync(join(SITE, 'static/CNAME'), 'utf8').trim();
const BASE = `https://${cname}${process.env.BASE_PATH ?? ''}`;

const ORIGIN = (() => {
	try {
		return execSync('git remote get-url origin', { cwd: REPO, encoding: 'utf8' })
			.trim()
			.replace(/\.git$/, '')
			.replace(/^git@github\.com:/, 'https://github.com/');
	} catch {
		return 'https://github.com/skamansam/gpui-forge';
	}
})();

// Mirrors CATEGORY_ORDER/PAGE_ORDER in src/lib/docs.ts — keep in sync.
const CATEGORY_ORDER = ['start', 'format', 'studio', 'web', 'cli'];
const PAGE_ORDER = {
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

const SUMMARY =
	'GPUI Forge is a declarative layer over GPUI (gpui.rs). A single `.forge.yaml` document ' +
	'describes a GPUI view and drives every tool in the ecosystem: the browser editor, the native ' +
	'Forge Studio workbench, preview hosts, and the `forge` CLI/compiler. The same document format ' +
	'targets multiple bases, including desktop (gpui-kit) and terminal (ratatui) rendering. The site ' +
	'renders real Forge documents live via the Rust renderers compiled to WebAssembly.';

// Curated links that aren't doc pages: app routes and external resources.
const LINK = (title, url, desc) => ({ title, url, desc });
const SECTIONS = [
	{
		h: 'Docs',
		head: [
			LINK('Documentation index', `${BASE}/docs`, 'Categorized index of all documentation pages with one-line summaries.')
		],
		cats: ['start']
	},
	{ h: 'The .forge.yaml format', cats: ['format'] },
	{ h: 'Forge Studio', cats: ['studio'] },
	{
		h: 'Web app',
		head: [
			LINK('Web editor', `${BASE}/editor`, 'The live browser editor — palette, element tree, inspector, live WASM canvas, and share links. No install.'),
			LINK('Examples', `${BASE}/examples`, 'The showcase — every shipped `examples/{demos,showcases}/*.forge.yaml` rendered beside its source, with roughly equivalent hand-written Rust.')
		],
		cats: ['web']
	},
	{
		h: 'CLI',
		cats: ['cli'],
		tail: [
			LINK('Releases', `${BASE}/releases`, 'Prebuilt `forge` CLI binaries per platform; releases are cut by tagging a commit `v*`.')
		]
	},
	{
		h: 'Optional',
		linksOnly: [
			LINK('forge-doc JSON Schema', `${BASE}/schema/forge-doc.schema.json`, 'The machine-readable JSON Schema used to validate `.forge.yaml` documents in editors.'),
			LINK('GitHub repository', ORIGIN, 'Source code, issue tracker, and the full `docs/` tree these pages are generated from.')
		]
	}
];

// Curated per-slug descriptions where the first paragraph isn't a good blurb
// (e.g. pages that open with a "Status: draft" line). Falls back to the
// extracted first paragraph otherwise.
const DESC = {
	'forge-yaml': 'The document syntax spec — plain YAML 1.2 constrained by a JSON Schema (draft spec v0.1).',
	'document-model': 'The canonical, format-independent AST that underlies the YAML/JSON representations and all tools.',
	'style-layout': 'The node `style:` map — flex layout, sizing, and presentation properties kept separate from component props.'
};

const stripInline = (s) =>
	s.replace(/\[([^\]]*)\]\([^)]*\)/g, '$1').replace(/[`*_]/g, '').trim();

function readDocs() {
	const docs = [];
	for (const cat of CATEGORY_ORDER) {
		const dir = join(REPO, 'docs', cat);
		for (const file of readdirSync(dir)) {
			if (!file.endsWith('.md')) continue;
			const slug = file.replace(/\.md$/, '');
			const md = readFileSync(join(dir, file), 'utf8');
			const title = md.match(/^# (.+)$/m)?.[1] ?? slug;
			const blurb =
				md
					.split(/\n{2,}/)
					.map((p) => p.trim())
					.find((p) => p && !p.startsWith('#') && !/^status:/i.test(p))
					?.replace(/\n/g, ' ') ?? '';
			let b = stripInline(blurb);
			if (b.length > 160) b = b.slice(0, 160).replace(/\s\S*$/, '') + '…';
			docs.push({ slug, title, blurb: b, md, cat });
		}
	}
	const order = (d) =>
		CATEGORY_ORDER.indexOf(d.cat) * 100 +
		((PAGE_ORDER[d.cat] ?? []).indexOf(d.slug) + 100) % 100;
	return docs.sort((a, b) => order(a) - order(b));
}

const docs = readDocs();
const byCat = (cat) => docs.filter((d) => d.cat === cat);
const line = (l) => `- [${l.title}](${l.url}): ${l.desc}`;
const docLine = (d) => `- [${d.title}](${BASE}/docs/${d.slug}): ${DESC[d.slug] ?? d.blurb}`;

// ---------- llms.txt: the curated index ----------
const index = [`# GPUI Forge`, '', `> ${SUMMARY}`, ''];
for (const s of SECTIONS) {
	index.push(`## ${s.h}`, '');
	for (const l of s.head ?? []) index.push(line(l));
	for (const c of s.cats ?? []) for (const d of byCat(c)) index.push(docLine(d));
	for (const l of s.tail ?? s.linksOnly ?? []) index.push(line(l));
	index.push('');
}

// ---------- llms-full.txt: every published doc inlined ----------
const fixLinks = (md, cat) =>
	md
		.replace(/\]\((?:\.\/)?([a-z0-9-]+)\.md(#[^)]*)?\)/g, (_, s, h = '') => `](${BASE}/docs/${s}${h})`)
		.replace(/\]\((\.\.?\/[^)]+)\)/g, (_, href) => {
			const t = normalize(join('docs', cat, href)).replaceAll('\\', '/');
			return `](${ORIGIN}/blob/main/${t})`;
		});

const demote = (md) =>
	md.replace(/^(#{2,6})\s+(.+)$/gm, (_, h, t) => '#'.repeat(Math.min(h.length + 1, 6)) + ' ' + t);

const full = [
	`# GPUI Forge`,
	'',
	`> ${SUMMARY}`,
	'',
	'> This file contains the full text of every published documentation page.',
	''
];
for (const d of docs) {
	const body = fixLinks(demote(d.md.replace(/^# .+\n/, '').trim()), d.cat);
	full.push(`## ${d.title}`, '', `Source: ${BASE}/docs/${d.slug}`, '', body, '', '---', '');
}

for (const [name, lines] of [
	['llms.txt', index],
	['llms-full.txt', full]
]) {
	const out = join(SITE, 'static', name);
	writeFileSync(out, lines.join('\n').replace(/\n{3,}/g, '\n\n').trim() + '\n');
	console.log(`${name}: ${docs.length} docs, ${lines.join('\n').split(/\s+/).length} words`);
}
