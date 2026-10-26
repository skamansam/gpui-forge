import { error } from '@sveltejs/kit';
import { docs } from '#lib/docs.ts';

export function entries() {
	return docs.map(({ slug }) => ({ slug }));
}

export function load({ params }) {
	const doc = docs.find((d) => d.slug === params.slug);
	if (!doc) error(404, 'No such page');
	return { doc, docs: docs.map(({ slug, title }) => ({ slug, title })) };
}
