<script lang="ts">
	import type { Snippet } from 'svelte';
	let { label, hint, error, for: id, children }: { label: string; hint?: string; error?: string; for?: string; children: Snippet } =
		$props();
</script>

<div class="flex flex-col gap-1.5">
	<label for={id} class="text-sm font-medium text-fg">{label}</label>
	{@render children()}
	{#if error}
		<p class="text-xs text-danger">{error}</p>
	{:else if hint}
		<p class="text-xs text-subtle">{hint}</p>
	{/if}
</div>
