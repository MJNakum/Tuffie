<script lang="ts">
	// Choose where a set applies: all workspaces, or a chosen list.
	import { api } from '#lib/api';
	import type { WorkspaceSummary } from '#lib/types';
	import Input from './ui/Input.svelte';
	import Check from '@lucide/svelte/icons/check';
	import Globe from '@lucide/svelte/icons/globe';
	import ListChecks from '@lucide/svelte/icons/list-checks';
	import { conceal, reveal } from '#lib/motion';

	let { global = $bindable(false), selected = $bindable<string[]>([]) }: { global?: boolean; selected?: string[] } = $props();
	let all = $state<WorkspaceSummary[]>([]);
	let q = $state('');
	$effect(() => {
		api.workspaces().then((w) => (all = w));
	});
	const shown = $derived(all.filter((w) => !q || w.name.toLowerCase().includes(q.toLowerCase())));
	function toggle(id: string) {
		selected = selected.includes(id) ? selected.filter((x) => x !== id) : [...selected, id];
	}
</script>

<div class="@container flex flex-col gap-3">
	<!-- Side by side when there is room; stacked in narrow panels. -->
	<div class="grid grid-cols-1 gap-3 @[440px]:grid-cols-2">
		{#each [{ g: true, icon: Globe, t: 'All workspaces', d: 'Applies to every workspace, now and later.' }, { g: false, icon: ListChecks, t: 'Selected workspaces', d: 'Applies only to the workspaces you choose.' }] as o (o.t)}
			<button
				type="button"
				onclick={() => (global = o.g)}
				class="relative flex items-start gap-3 rounded-lg border p-3 text-left transition-colors {global === o.g ? 'border-primary bg-primary-soft' : 'border-border-strong hover:bg-hover'}"
			>
				<o.icon class="mt-0.5 size-4 {global === o.g ? 'text-primary-text' : 'text-muted'}" />
				<div>
					<div class="text-base font-medium">{o.t}</div>
					<div class="mt-0.5 text-xs text-muted">{o.d}</div>
				</div>
			</button>
		{/each}
	</div>
	{#if !global}
		<div class="overflow-hidden rounded-lg border border-border-strong" in:reveal out:conceal>
			<div class="border-b border-border p-2"><Input bind:value={q} placeholder="Filter workspaces" class="h-7" /></div>
			<div class="max-h-56 overflow-auto py-1">
				{#each shown as w (w.id)}
					{@const on = selected.includes(w.id)}
					<button type="button" class="flex w-full items-center gap-2.5 px-3 py-1.5 text-left text-base hover:bg-hover" onclick={() => toggle(w.id)}>
						<span class="grid size-4 place-items-center rounded border transition-colors duration-(--dur-fast) {on ? 'border-primary bg-primary text-white' : 'border-border-strong'}">
							{#if on}<Check class="size-3" strokeWidth={3} />{/if}
						</span>
						<span class="flex-1">{w.name}</span>
						<span class="mono text-2xs text-subtle">{w.tool}</span>
					</button>
				{:else}
					<p class="px-3 py-3 text-sm text-subtle">No workspaces.</p>
				{/each}
			</div>
			<div class="border-t border-border bg-surface-2 px-3 py-1.5 text-xs text-muted">{selected.length} selected</div>
		</div>
	{/if}
</div>
