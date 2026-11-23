// In-app documentation. The pages are the Markdown files in docs/guide,
// bundled at build time, so the app and GitHub show the same text.
import { Marked, type Tokens } from 'marked';
import DOMPurify from 'dompurify';

const raw = import.meta.glob('../../../docs/guide/*.md', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;

export interface DocPage {
	slug: string;
	title: string;
	section: string;
	markdown: string;
	/** Plain text, for search. */
	text: string;
}

const ORDER: { section: string; slugs: string[] }[] = [
	{ section: 'Start here', slugs: ['README', 'getting-started', 'concepts'] },
	{ section: 'Use Tuffie', slugs: ['workspaces', 'runs', 'state', 'terminal', 'files', 'variables', 'policies', 'drift'] },
	{ section: 'Run Tuffie', slugs: ['configuration', 'security', 'shortcuts', 'troubleshooting'] }
];

function slugOf(path: string) {
	return path.split('/').pop()!.replace(/\.md$/, '');
}

const bySlug = new Map<string, string>(Object.entries(raw).map(([p, md]) => [slugOf(p), md]));

export const pages: DocPage[] = ORDER.flatMap(({ section, slugs }) =>
	slugs
		.filter((s) => bySlug.has(s))
		.map((slug) => {
			const markdown = bySlug.get(slug)!;
			const title = slug === 'README' ? 'Overview' : (markdown.match(/^#\s+(.+)$/m)?.[1] ?? slug);
			const text = markdown
				.replace(/```[\s\S]*?```/g, ' ')
				.replace(/[#*`>|_[\]()-]/g, ' ')
				.replace(/\s+/g, ' ');
			return { slug, title, section, markdown, text };
		})
);

export const sections = ORDER.map((o) => ({ section: o.section, pages: pages.filter((p) => p.section === o.section) }));

export function headingId(text: string) {
	return text
		.toLowerCase()
		.replace(/<[^>]+>/g, '')
		.replace(/[^a-z0-9\s-]/g, '')
		.trim()
		.replace(/\s+/g, '-');
}

/** Map a link in a guide page to an in-app URL. */
function rewriteHref(href: string): string {
	if (/^[a-z]+:/i.test(href) || href.startsWith('#') || href.startsWith('/')) return href;
	const m = href.match(/^(?:\.\/)?([\w-]+)\.md(#.*)?$/);
	if (m) return `/docs/${m[1] === 'README' ? '' : m[1]}${m[2] ?? ''}`;
	// Links outside the guide (for example ../ARCHITECTURE.md) go to GitHub.
	const out = href.replace(/^\.\.\//, '');
	return `https://github.com/MJNakum/Tuffie/blob/main/docs/${out}`;
}

export interface Rendered {
	html: string;
	toc: { id: string; text: string; depth: number }[];
}

export function render(markdown: string): Rendered {
	const toc: Rendered['toc'] = [];
	const marked = new Marked({
		gfm: true,
		renderer: {
			heading(this: any, { tokens, depth }: Tokens.Heading) {
				const text = this.parser.parseInline(tokens);
				const id = headingId(text);
				if (depth === 2 || depth === 3) toc.push({ id, text: text.replace(/<[^>]+>/g, ''), depth });
				return `<h${depth} id="${id}"><a class="anchor" href="#${id}" aria-hidden="true">#</a>${text}</h${depth}>`;
			},
			link(this: any, { href, title, tokens }: Tokens.Link) {
				const text = this.parser.parseInline(tokens);
				const url = rewriteHref(href);
				const ext = /^https?:/.test(url);
				return `<a href="${url}"${title ? ` title="${title}"` : ''}${ext ? ' target="_blank" rel="noreferrer"' : ''}>${text}</a>`;
			},
			blockquote(this: any, { tokens }: Tokens.Blockquote) {
				const body = this.parser.parse(tokens);
				const kind = /<strong>Caution:<\/strong>/.test(body) ? 'caution' : /<strong>Note:<\/strong>/.test(body) ? 'note' : 'quote';
				return `<blockquote class="callout callout-${kind}">${body}</blockquote>`;
			}
		}
	});
	const html = DOMPurify.sanitize(marked.parse(markdown) as string, { ADD_ATTR: ['target'] });
	return { html, toc };
}

export function search(q: string): { page: DocPage; snippet: string }[] {
	const query = q.trim().toLowerCase();
	if (query.length < 2) return [];
	const out: { page: DocPage; snippet: string; score: number }[] = [];
	for (const page of pages) {
		const inTitle = page.title.toLowerCase().includes(query);
		const i = page.text.toLowerCase().indexOf(query);
		if (!inTitle && i < 0) continue;
		const start = Math.max(0, i - 50);
		const snippet = i < 0 ? page.text.slice(0, 110) : (start > 0 ? '…' : '') + page.text.slice(start, i + query.length + 70) + '…';
		out.push({ page, snippet, score: inTitle ? 2 : 1 });
	}
	return out.sort((a, b) => b.score - a.score).slice(0, 8);
}
