import { marked } from 'marked';
import { resolve } from '$app/paths';

const files = import.meta.glob('../../../docs/spec/*.md', {
	query: '?raw',
	import: 'default',
	eager: true
}) as Record<string, string>;

const REPO_SPEC = 'https://github.com/skamansam/gpui-forge/blob/main/docs/spec/';

export type Doc = { slug: string; title: string; html: string };

export const docs: Doc[] = Object.entries(files)
	.map(([path, md]) => {
		const slug = path.split('/').pop()!.replace(/\.md$/, '');
		const title = md.match(/^# (.+)$/m)?.[1] ?? slug;
		// Links between spec files become links between doc pages.
		const html = (marked.parse(md, { async: false }) as string).replace(
			/href="(?:\.\/)?([a-z0-9-]+)\.md(#[^"]*)?"/g,
			(_, s, hash = '') => `href="${resolve('/docs/[slug]', { slug: s })}${hash}"`
		).replace(
			// Other relative links point at files in the repository.
			/href="((?:\.\.?\/)[^"]*)"/g,
			(_, href) => `href="${new URL(href, REPO_SPEC).href}"`
		);
		return { slug, title, html };
	})
	.sort((a, b) => order(a.slug) - order(b.slug));

function order(slug: string) {
	const i = ['forge-yaml', 'document-model', 'catalog-manifest'].indexOf(slug);
	return i < 0 ? 99 : i;
}
