<script lang="ts">
	import type { Snippet } from 'svelte';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	let {
		crumbs = [],
		title,
		subtitle,
		actions,
		meta
	}: { crumbs?: { label: string; href?: string }[]; title: string; subtitle?: string; actions?: Snippet; meta?: Snippet } = $props();
</script>

<header class="shrink-0 border-b border-border bg-surface px-8 pt-5">
	{#if crumbs.length}
		<nav class="mb-2 flex items-center gap-1 text-[12px] text-subtle">
			{#each crumbs as c, i (i)}
				{#if c.href}<a href={c.href} class="hover:text-fg">{c.label}</a>{:else}<span>{c.label}</span>{/if}
				{#if i < crumbs.length - 1}<ChevronRight class="size-3" />{/if}
			{/each}
		</nav>
	{/if}
	<div class="flex items-start justify-between gap-6 pb-4">
		<div class="min-w-0">
			<h1 class="truncate text-[20px] font-semibold tracking-[-0.02em]">{title}</h1>
			{#if subtitle}<p class="mt-0.5 text-[13px] text-muted">{subtitle}</p>{/if}
			{#if meta}<div class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-[12px] text-muted">{@render meta()}</div>{/if}
		</div>
		{#if actions}<div class="flex shrink-0 items-center gap-2">{@render actions()}</div>{/if}
	</div>
</header>
