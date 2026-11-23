<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	type Variant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'outline-danger';
	type Size = 'sm' | 'md' | 'lg' | 'icon' | 'icon-sm';

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

	// Sizes: sm 28 px (toolbars, tables), md 32 px (default), lg 36 px (main
	// call to action on empty pages). Icon buttons are square.
	const base =
		'pressable relative inline-flex items-center justify-center gap-1.5 whitespace-nowrap rounded-md font-medium select-none disabled:pointer-events-none disabled:opacity-50 [&_svg]:shrink-0';
	const variants: Record<Variant, string> = {
		primary:
			'bg-primary text-primary-fg hover:bg-primary-hover active:bg-primary-press shadow-[inset_0_1px_0_rgb(255_255_255/0.12),0_1px_2px_rgb(0_0_0/0.2)] hover:shadow-[inset_0_1px_0_rgb(255_255_255/0.14),0_2px_8px_-2px_var(--primary)]',
		secondary: 'bg-surface-2 text-fg border border-border-strong hover:bg-hover hover:border-[color-mix(in_srgb,var(--border-strong),var(--fg)_12%)]',
		ghost: 'text-muted hover:text-fg hover:bg-hover',
		danger: 'bg-danger text-white hover:brightness-110',
		'outline-danger': 'border border-border-strong text-danger hover:bg-danger-soft hover:border-danger/40'
	};
	const sizes: Record<Size, string> = {
		sm: 'h-7 px-2.5 text-sm [&_svg]:size-3.5',
		md: 'h-8 px-3 text-base [&_svg]:size-4',
		lg: 'h-9 px-4 text-base [&_svg]:size-4',
		icon: 'size-8 [&_svg]:size-4',
		'icon-sm': 'size-7 [&_svg]:size-3.5'
	};
	const c = $derived(`${base} ${variants[variant]} ${sizes[size]} ${cls}`);
</script>

{#snippet content()}
	<!-- The label keeps its space while loading, so the button never changes width. -->
	<span class="inline-flex items-center gap-1.5 transition-opacity duration-(--dur-fast) {loading ? 'opacity-0' : 'opacity-100'}">
		{@render children?.()}
	</span>
	{#if loading}
		<span class="motion-fade absolute inset-0 grid place-items-center">
			<svg class="animate-spin" viewBox="0 0 24 24" fill="none"
				><circle cx="12" cy="12" r="9" stroke="currentColor" stroke-opacity=".25" stroke-width="3" /><path
					d="M21 12a9 9 0 0 0-9-9"
					stroke="currentColor"
					stroke-width="3"
					stroke-linecap="round"
				/></svg
			>
		</span>
	{/if}
{/snippet}

{#if href}
	<a {href} class={c} {...rest as Record<string, unknown>}>{@render content()}</a>
{:else}
	<button class={c} disabled={loading || rest.disabled} aria-busy={loading || undefined} {...rest}>{@render content()}</button>
{/if}
