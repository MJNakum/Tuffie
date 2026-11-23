<script lang="ts">
	import type { Snippet } from 'svelte';
	let { icon: Icon, title, text, children }: { icon?: any; title: string; text?: string; children?: Snippet } = $props();
</script>

<div class="motion-rise flex flex-col items-center justify-center px-6 py-14 text-center">
	{#if Icon}
		<div class="mb-4 grid size-10 place-items-center rounded-lg border border-border-strong bg-surface-2 text-muted">
			<Icon class="size-5" />
		</div>
	{/if}
	<h3 class="text-md font-semibold">{title}</h3>
	{#if text}<p class="mt-1 max-w-sm text-base text-muted">{text}</p>{/if}
	{#if children}<div class="mt-5 flex gap-2">{@render children()}</div>{/if}
</div>
