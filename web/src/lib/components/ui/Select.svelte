<script lang="ts">
	import type { HTMLSelectAttributes } from 'svelte/elements';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	let {
		value = $bindable(),
		options,
		class: cls = '',
		...rest
	}: HTMLSelectAttributes & { options: { value: string; label: string }[] } = $props();
</script>

<div class="relative {cls}">
	<select
		bind:value
		class="h-8 w-full appearance-none rounded-md border border-border-strong bg-surface pl-2.5 pr-8 text-[13px] text-fg focus:border-primary focus:outline-none focus:ring-3 focus:ring-primary-soft"
		{...rest}
	>
		{#each options as o (o.value)}
			<option value={o.value}>{o.label}</option>
		{/each}
	</select>
	<ChevronDown class="pointer-events-none absolute right-2.5 top-1/2 size-3.5 -translate-y-1/2 text-subtle" />
</div>
