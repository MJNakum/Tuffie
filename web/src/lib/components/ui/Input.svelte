<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';
	let { value = $bindable(), class: cls = '', mono = false, ...rest }: HTMLInputAttributes & { mono?: boolean } = $props();
</script>

<input
	bind:value
	class="h-8 w-full rounded-md border border-border-strong bg-surface px-2.5 text-base text-fg placeholder:text-subtle transition-colors hover:border-[color-mix(in_srgb,var(--border-strong),var(--fg)_15%)] focus:border-primary focus:outline-none focus:ring-3 focus:ring-primary-soft transition-[border-color,box-shadow,background-color] duration-(--dur-fast) disabled:opacity-60 {mono
		? 'font-mono text-sm'
		: ''} {cls}"
	{...rest}
/>
