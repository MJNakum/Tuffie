<script lang="ts">
	import type { Snippet } from 'svelte';
	let { children }: { children: Snippet } = $props();
</script>

<kbd class="inline-flex h-[18px] min-w-[18px] items-center justify-center rounded border border-border-strong bg-surface-2 px-1 font-sans text-[10.5px] font-medium text-muted">{@render children()}</kbd>
