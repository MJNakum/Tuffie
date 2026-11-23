<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	type Variant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'outline-danger';
	type Size = 'sm' | 'md' | 'icon' | 'icon-sm';

	let {
		variant = 'secondary',
		size = 'md',
		href,
		loading = false,
		class: cls = '',
		children,
		...rest
	}: HTMLButtonAttributes & {
		variant?: Variant;
		size?: Size;
		href?: string;
		loading?: boolean;
		children?: Snippet;
	} = $props();

	const base =
		'inline-flex items-center justify-center gap-1.5 whitespace-nowrap rounded-md font-medium select-none transition-colors duration-100 disabled:pointer-events-none disabled:opacity-50 [&_svg]:shrink-0';
	const variants: Record<Variant, string> = {
		primary: 'bg-primary text-primary-fg hover:bg-primary-hover active:bg-primary-press shadow-[inset_0_1px_0_rgb(255_255_255/0.12)]',
		secondary: 'bg-surface-2 text-fg border border-border-strong hover:bg-hover',
		ghost: 'text-muted hover:text-fg hover:bg-hover',
		danger: 'bg-danger text-white hover:brightness-110',
		'outline-danger': 'border border-border-strong text-danger hover:bg-danger-soft'
	};
	const sizes: Record<Size, string> = {
		sm: 'h-7 px-2.5 text-[12.5px] [&_svg]:size-3.5',
		md: 'h-8 px-3 text-[13px] [&_svg]:size-4',
		icon: 'size-8 [&_svg]:size-4',
		'icon-sm': 'size-7 [&_svg]:size-3.5'
	};
	const c = $derived(`${base} ${variants[variant]} ${sizes[size]} ${cls}`);
</script>

{#if href}
	<a {href} class={c} {...rest as Record<string, unknown>}>{@render children?.()}</a>
{:else}
	<button class={c} disabled={loading || rest.disabled} {...rest}>
		{#if loading}
			<svg class="animate-spin" viewBox="0 0 24 24" fill="none"
				><circle cx="12" cy="12" r="9" stroke="currentColor" stroke-opacity=".25" stroke-width="3" /><path
					d="M21 12a9 9 0 0 0-9-9"
					stroke="currentColor"
					stroke-width="3"
					stroke-linecap="round"
				/></svg
			>
		{/if}
		{@render children?.()}
	</button>
{/if}
