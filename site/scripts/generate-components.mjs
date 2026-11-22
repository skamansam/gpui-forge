// Generates docs/format/components.md — the supported-components status page —
// from ../../ALL-COMPONENTS.md (the audit checklist every adapter/catalog is
// reconciled against). Runs via `prebuild`/`predev` next to generate-llms.mjs
// so the page always reflects the real support matrix. The output file is
// committed like the schemas — regenerate it whenever ALL-COMPONENTS.md
// changes. Do not edit it by hand.

import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO = fileURLToPath(new URL('../..', import.meta.url)); // repo root
const SRC = join(REPO, 'ALL-COMPONENTS.md');
const OUT = join(REPO, 'docs/format/components.md');

const src = readFileSync(SRC, 'utf8').split('\n');

// How each set is enabled — `use:` for adapter libraries, `base:` for
// rendering bases. Sections whose note already names a `use:` alias use it.
// the rest fall back to this map.
const ENABLE = {
	'GPUI-KIT Components': '`base: gpui-kit` (default)',
	'GPUI-KIT sub-elements (catalog)': 'inside gpui-kit parents',
	'guise Components': '`base: guise`',
	'ratatui Components': '`base: ratatui`',
	'iced Components': '`base: iced`',
	'bezel Components': '`base: bezel` (scaffold)',
	'adabraka Components': 'catalog only (scaffold)',
	'forge built-ins': 'always available',
	'template-lib Components': 'adapter scaffold'
};

// --- Parse ALL-COMPONENTS.md into sections ---------------------------------
// A section starts at `## Name`, may carry `docs:`/`github:` links and a `>`
// note, then `- [x]` / `- [ ]` checklist entries. An unchecked entry's reason
// follows `—` on the same line.
const sections = [];
let cur = null;
for (const line of src) {
	const h = line.match(/^## (.+)/);
	if (h) {
		cur = { name: h[1].trim(), links: {}, notes: [], done: [], skipped: [] };
		sections.push(cur);
		continue;
	}
	if (!cur) continue;
	const kv = line.match(/^(docs|github):\s+(\S+)/);
	if (kv) {
		cur.links[kv[1]] = kv[2];
		continue;
	}
	if (line.startsWith('>')) {
		cur.notes.push(line.slice(1).trim());
		continue;
	}
	const item = line.match(/^- \[(x| )\] (`?)([\w.]+)\2\s*(?:\(([^)]*)\))?\s*(?:—\s*(.*))?$/);
	if (item) {
		const [, mark, , name, parens, reason] = item;
		// `(…)` is a URL only for http(s) links — otherwise it's an inline note
		// like "(alias of Switch)".
		const entry = {
			name,
			link: /^https?:/.test(parens ?? '') ? parens : undefined,
			note: /^https?:/.test(parens ?? '') ? undefined : parens,
			reason: (reason ?? '').trim()
		};
		(mark === 'x' ? cur.done : cur.skipped).push(entry);
	}
}

const sets = sections.filter((s) => s.done.length + s.skipped.length > 0);

// --- Emit -------------------------------------------------------------------
const esc = (s) => s.replace(/\|/g, '\\|');
const useHint = (s) => {
	const note = s.notes.find((n) => /\buse:/.test(n));
	if (!note) return '';
	// Notes write the alias in backticks — `` `use: ely: ely-core` `` — but a
	// few use bare `use:` text. Try the backticked form first.
	const m = note.match(/`(use:[^`]+)`/) ?? note.match(/use:\s*[^\n—,)]+/);
	return m ? `\`${m[1] ?? m[0]}\``.replace(/`{2,}/g, '`') : '';
};
const enable = (s) => useHint(s) || ENABLE[s.name] || '—';
const anchor = (name) =>
	name
		.toLowerCase()
		.replace(/[^\p{L}\p{N}]+/gu, '-')
		.replace(/^-|-$/g, '');

const totalDone = sets.reduce((n, s) => n + s.done.length, 0);
const totalSkip = sets.reduce((n, s) => n + s.skipped.length, 0);

let md = `# Supported components

> Generated from [ALL-COMPONENTS.md](https://github.com/skamansam/gpui-forge/blob/main/ALL-COMPONENTS.md) by \`site/scripts/generate-components.mjs\` — do not edit by hand.

Every component set Forge understands, what each provides, and how to enable it. A document picks a rendering base with \`base:\` and pulls adapter libraries in through \`use:\` imports — see [adapters](adapters.md).

**${totalDone} supported elements** across ${sets.length} sets. ${totalSkip} upstream exports were reviewed and deliberately not exposed — each carries a reason below (value types, traits, engine handles, or runtime requirements a declarative document cannot express).

| Set | Enable | Supported | Reviewed — not widgets |
| --- | --- | --- | --- |
`;

for (const s of sets) {
	md += `| [${s.name}](#${anchor(s.name)}) | ${enable(s)} | ${s.done.length} | ${s.skipped.length} |\n`;
}

for (const s of sets) {
	md += `\n## ${s.name}\n\n`;
	const links = [
		s.links.docs && `[docs](${s.links.docs})`,
		s.links.github && `[github](${s.links.github})`
	].filter(Boolean);
	if (links.length) md += links.join(' · ') + '\n\n';
	for (const note of s.notes) md += `> ${note}\n\n`;

	if (s.done.length) {
		md += `<details><summary><strong>${s.done.length} supported elements</strong></summary>\n\n`;
		md += s.done
			.map((e) => {
				const name = e.link ? `[\`${e.name}\`](${e.link})` : `\`${e.name}\``;
				const suffix = e.note ?? (e.reason || undefined);
				return suffix ? `${name} (${suffix})` : name;
			})
			.join(' ');
		md += '\n\n</details>\n\n';
	}
	if (s.skipped.length) {
		md += `<details><summary><strong>${s.skipped.length} reviewed — not standalone widgets</strong></summary>\n\n`;
		md += '| Export | Reason |\n| --- | --- |\n';
		for (const e of s.skipped) md += `| \`${e.name}\` | ${esc(e.reason || '—')} |\n`;
		md += '\n</details>\n';
	}
}

writeFileSync(OUT, md);
console.log(`wrote ${OUT}: ${sets.length} sets, ${totalDone} supported, ${totalSkip} reviewed`);
