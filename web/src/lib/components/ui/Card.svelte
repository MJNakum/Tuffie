<script lang="ts">
	import type { Snippet } from 'svelte';
	let {
		title,
		actions,
		class: cls = '',
		pad = true,
		tour,
		children
	}: { title?: string; actions?: Snippet; class?: string; pad?: boolean; tour?: string; children: Snippet } = $props();
</script>

<section class="rounded-lg border border-border bg-surface {cls}" data-tour={tour}>
	{#if title || actions}
		<header class="flex h-11 items-center justify-between gap-3 border-b border-border px-4">
			<h3 class="text-base font-semibold">{title}</h3>
			{#if actions}<div class="flex items-center gap-2">{@render actions()}</div>{/if}
		</header>
	{/if}
	<div class={pad ? 'p-4' : ''}>{@render children()}</div>
</section>
