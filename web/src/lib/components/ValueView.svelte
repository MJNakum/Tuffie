<script lang="ts">
	// Renders a Terraform value (JSON) compactly with types coloured.
	import ValueView from './ValueView.svelte';
	let { value, unknown = false }: { value: any; unknown?: boolean } = $props();
	const isObj = $derived(value !== null && typeof value === 'object');
</script>

{#if unknown}
	<span class="italic text-subtle">(known after apply)</span>
{:else if value === null || value === undefined}
	<span class="text-subtle">null</span>
{:else if value === '(sensitive value)'}
	<span class="rounded bg-surface-3 px-1 text-[11.5px] text-muted">sensitive</span>
{:else if typeof value === 'string'}
	<span class="break-all text-[#9ecbff] [[data-theme=light]_&]:text-[#0a5bb5]">"{value}"</span>
{:else if typeof value === 'number'}
	<span class="text-[#f5b370] [[data-theme=light]_&]:text-[#b35900]">{value}</span>
{:else if typeof value === 'boolean'}
	<span class="text-violet">{value}</span>
{:else if isObj && Array.isArray(value)}
	{#if value.length === 0}<span class="text-subtle">[]</span>{:else}
		<span class="text-subtle">[</span>
		<div class="ml-4">
			{#each value as v, i (i)}<div><ValueView value={v} />{#if i < value.length - 1}<span class="text-subtle">,</span>{/if}</div>{/each}
		</div>
		<span class="text-subtle">]</span>
	{/if}
{:else if isObj}
	{@const keys = Object.keys(value)}
	{#if keys.length === 0}<span class="text-subtle">{'{}'}</span>{:else}
		<span class="text-subtle">{'{'}</span>
		<div class="ml-4">
			{#each keys as k (k)}<div><span class="text-fg">{k}</span><span class="text-subtle"> = </span><ValueView value={value[k]} /></div>{/each}
		</div>
		<span class="text-subtle">{'}'}</span>
	{/if}
{/if}
