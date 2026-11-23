<script lang="ts">
	import { indicator } from '#lib/motion';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { tick } from 'svelte';
	import { pages, render, search, sections } from '#lib/docs';
	import Input from '#lib/components/ui/Input.svelte';
	import Search from '@lucide/svelte/icons/search';
	import BookOpen from '@lucide/svelte/icons/book-open';
	import ChevronLeft from '@lucide/svelte/icons/chevron-left';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import Compass from '@lucide/svelte/icons/compass';
	import { startTour } from '#lib/tour';

	const slug = $derived(page.params.slug ?? 'README');
	const doc = $derived(pages.find((p) => p.slug === slug) ?? null);
	const rendered = $derived(doc ? render(doc.markdown) : null);
	const index = $derived(pages.findIndex((p) => p.slug === slug));
	const prev = $derived(index > 0 ? pages[index - 1] : null);
	const next = $derived(index >= 0 && index < pages.length - 1 ? pages[index + 1] : null);

	let q = $state('');
	const results = $derived(search(q));
	let scroller: HTMLDivElement | undefined = $state();

	// Scroll to the anchor, or to the top, when the page changes.
	$effect(() => {
		slug;
		const hash = page.url.hash;
		tick().then(() => {
			const el = hash ? document.getElementById(decodeURIComponent(hash.slice(1))) : null;
			if (el) el.scrollIntoView();
			else scroller?.scrollTo(0, 0);
		});
	});

	const href = (s: string) => (s === 'README' ? '/docs' : `/docs/${s}`);
</script>

<svelte:head><title>{doc?.title ?? 'Docs'} · Tuffie docs</title></svelte:head>

<div class="flex min-h-0 flex-1">
	<aside class="flex w-64 shrink-0 flex-col border-r border-border bg-surface">
		<div class="border-b border-border p-3">
			<div class="relative">
				<Search class="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-subtle" />
				<Input bind:value={q} placeholder="Search the docs" class="pl-8" data-tour="docs-search" />
			</div>
		</div>
		<nav class="min-h-0 flex-1 overflow-auto px-2 py-3">
			{#if q.trim().length >= 2}
				<div class="px-2 pb-2 text-2xs font-medium uppercase tracking-wide text-subtle">{results.length} result{results.length === 1 ? '' : 's'}</div>
				{#each results as r (r.page.slug)}
					<a href={href(r.page.slug)} onclick={() => (q = '')} class="mb-1 block rounded-md px-2 py-2 hover:bg-hover">
						<div class="text-base font-medium">{r.page.title}</div>
						<div class="mt-0.5 line-clamp-2 text-xs text-muted">{r.snippet}</div>
					</a>
				{:else}
					<p class="px-2 text-sm text-subtle">Nothing found for “{q}”.</p>
				{/each}
			{:else}
				<div class="relative" use:indicator>
				<span data-indicator class="pointer-events-none absolute left-0 top-0 rounded-md bg-primary-soft opacity-0"></span>
				{#each sections as s (s.section)}
					<div class="mb-4">
						<div class="px-2 pb-1 text-2xs font-medium uppercase tracking-wide text-subtle">{s.section}</div>
						{#each s.pages as p (p.slug)}
							<a
								href={href(p.slug)}
								data-active={p.slug === slug}
								class="relative z-10 block rounded-md px-2 py-1.5 text-base {p.slug === slug ? 'font-medium text-primary-text' : 'text-muted hover:bg-hover hover:text-fg'}"
								>{p.title}</a
							>
						{/each}
					</div>
				{/each}
				</div>
			{/if}
		</nav>
		<div class="border-t border-border p-3">
			<button class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-base text-muted hover:bg-hover hover:text-fg" onclick={() => (goto('/'), setTimeout(() => startTour('welcome', true), 400))}>
				<Compass class="size-4" />Take the guided tour
			</button>
		</div>
	</aside>

	<div bind:this={scroller} class="min-h-0 flex-1 overflow-auto">
		{#if !doc || !rendered}
			<div class="grid place-items-center py-24 text-center">
				<BookOpen class="mb-3 size-6 text-subtle" />
				<p class="text-md font-medium">Page not found</p>
				<a href="/docs" class="mt-2 text-base text-primary-text hover:underline">Go to the overview</a>
			</div>
		{:else}
			<div class="mx-auto flex max-w-[1100px] gap-10 px-10 py-10">
				<article class="doc min-w-0 flex-1">
					<div class="mb-2 text-xs font-medium text-primary-text">{doc.section}</div>
					{@html rendered.html}
					<nav class="mt-14 grid grid-cols-2 gap-3 border-t border-border pt-6">
						{#if prev}
							<a href={href(prev.slug)} class="group rounded-lg border border-border p-3 hover:border-primary">
								<div class="flex items-center gap-1 text-2xs text-subtle"><ChevronLeft class="size-3" />Previous</div>
								<div class="mt-0.5 text-md font-medium group-hover:text-primary-text">{prev.title}</div>
							</a>
						{:else}<span></span>{/if}
						{#if next}
							<a href={href(next.slug)} class="group rounded-lg border border-border p-3 text-right hover:border-primary">
								<div class="flex items-center justify-end gap-1 text-2xs text-subtle">Next<ChevronRight class="size-3" /></div>
								<div class="mt-0.5 text-md font-medium group-hover:text-primary-text">{next.title}</div>
							</a>
						{/if}
					</nav>
				</article>
				{#if rendered.toc.length > 2}
					<aside class="sticky top-10 hidden h-fit w-52 shrink-0 xl:block">
						<div class="mb-2 text-2xs font-medium uppercase tracking-wide text-subtle">On this page</div>
						<ul class="flex flex-col gap-1 border-l border-border">
							{#each rendered.toc as h (h.id)}
								<li><a href="#{h.id}" class="-ml-px block border-l border-transparent py-0.5 text-sm text-muted hover:border-primary hover:text-fg {h.depth === 3 ? 'pl-6' : 'pl-3'}">{h.text}</a></li>
							{/each}
						</ul>
					</aside>
				{/if}
			</div>
		{/if}
	</div>
</div>
