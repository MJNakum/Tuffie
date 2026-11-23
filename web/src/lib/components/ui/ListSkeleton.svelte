<script lang="ts">
	// Placeholder for a table or list card: same row height as the real rows.
	import Skeleton from './Skeleton.svelte';
	let { rows = 4, header = true }: { rows?: number; header?: boolean } = $props();
</script>

<div class="overflow-hidden rounded-lg border border-border bg-surface" aria-busy="true" aria-label="Loading">
	{#if header}<div class="h-10 border-b border-border bg-surface-2"></div>{/if}
	{#each { length: rows } as _, i (i)}
		<div class="flex h-14 items-center gap-6 border-b border-border px-4 last:border-b-0" style="opacity: {1 - i * 0.16}">
			<Skeleton class="h-3 w-40" />
			<Skeleton class="h-3 w-24" />
			<Skeleton class="ml-auto h-3 w-16" />
		</div>
	{/each}
</div>
