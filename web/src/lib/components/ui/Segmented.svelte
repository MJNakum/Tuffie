<script lang="ts" generics="T extends string">
	// Segmented control with a thumb that slides to the selected option.
	import { indicator } from '#lib/motion';
	import type { Snippet } from 'svelte';

	let {
		value = $bindable(),
		options,
		size = 'md',
		item
	}: {
		value: T;
		options: { value: T; label: string; count?: number }[];
		size?: 'sm' | 'md';
		item?: Snippet<[{ value: T; label: string; count?: number }]>;
	} = $props();
</script>

<div class="relative inline-flex w-fit items-center rounded-md bg-surface-2 p-0.5" role="tablist" use:indicator>
	<span data-indicator class="pointer-events-none absolute left-0 top-0 rounded-[5px] bg-surface-3 shadow-[0_1px_2px_rgb(0_0_0/0.25)] opacity-0"></span>
	{#each options as o (o.value)}
		<button
			type="button"
			role="tab"
			aria-selected={value === o.value}
			data-active={value === o.value}
			onclick={() => (value = o.value)}
			class="relative z-10 rounded-[5px] font-medium {size === 'sm' ? 'h-6 px-2 text-xs' : 'h-7 px-3 text-sm'} {value === o.value ? 'text-fg' : 'text-muted hover:text-fg'}"
		>
			{#if item}{@render item(o)}{:else}{o.label}{#if o.count !== undefined}<span class="ml-1 tabular text-subtle">{o.count}</span>{/if}{/if}
		</button>
	{/each}
</div>
