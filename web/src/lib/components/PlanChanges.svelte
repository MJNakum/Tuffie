<script lang="ts">
	import { actionInfo, actionOf, toneBg, toneText } from '#lib/format';
	import type { Plan, ResourceChange } from '#lib/types';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import ValueView from './ValueView.svelte';
	import Input from './ui/Input.svelte';

	let { plan }: { plan: Plan } = $props();
	let filter = $state('');
	let only = $state<string>('all');
	let open = $state<Record<string, boolean>>({});

	const changes = $derived(plan.resource_changes.filter((c) => actionOf(c.actions) !== 'no-op'));
	const counts = $derived(
		changes.reduce(
			(acc, c) => {
				acc[actionOf(c.actions)] = (acc[actionOf(c.actions)] ?? 0) + 1;
				return acc;
			},
			{} as Record<string, number>
		)
	);
	const shown = $derived(
		changes.filter(
			(c) => (only === 'all' || actionOf(c.actions) === only) && (!filter || c.address.toLowerCase().includes(filter.toLowerCase()))
		)
	);

	type Row = { key: string; before: any; after: any; unknown: boolean; kind: 'add' | 'remove' | 'change' | 'same' };

	function rows(c: ResourceChange): Row[] {
		const b = (c.before ?? {}) as Record<string, any>;
		const a = (c.after ?? {}) as Record<string, any>;
		const u = (c.after_unknown ?? {}) as Record<string, any>;
		const keys = [...new Set([...Object.keys(b), ...Object.keys(a), ...Object.keys(u)])].sort();
		return keys.map((k) => {
			const unknown = u[k] === true;
			const inB = k in b && b[k] !== null;
			const inA = (k in a && a[k] !== null) || unknown;
			const same = !unknown && JSON.stringify(b[k]) === JSON.stringify(a[k]);
			const kind: Row['kind'] = same ? 'same' : !inB ? 'add' : !inA ? 'remove' : 'change';
			return { key: k, before: b[k], after: a[k], unknown, kind };
		});
	}
	const sym = { add: '+', remove: '−', change: '~', same: ' ' };
	const symTone = { add: 'text-success', remove: 'text-danger', change: 'text-warning', same: 'text-subtle' };
</script>

<div class="flex flex-wrap items-center gap-2 border-b border-border px-4 py-2.5" data-tour="plan-filter">
	<div class="flex items-center gap-1 rounded-md bg-surface-2 p-0.5">
		{#each ['all', 'create', 'update', 'replace', 'delete', 'read'] as k (k)}
			{#if k === 'all' || counts[k]}
				<button
					class="h-6 rounded px-2 text-[12px] font-medium transition-colors {only === k ? 'bg-surface-3 text-fg shadow-sm' : 'text-muted hover:text-fg'}"
					onclick={() => (only = k)}
				>
					{k === 'all' ? 'All' : actionInfo[k as keyof typeof actionInfo].label}
					<span class="ml-0.5 tabular text-subtle">{k === 'all' ? changes.length : counts[k]}</span>
				</button>
			{/if}
		{/each}
	</div>
	<div class="ml-auto w-64"><Input placeholder="Filter by address" bind:value={filter} class="h-7" /></div>
</div>

{#if changes.length === 0}
	<p class="px-4 py-8 text-center text-[13px] text-muted">No changes. Your infrastructure matches the configuration.</p>
{/if}

<ul class="divide-y divide-border">
	{#each shown as c (c.address)}
		{@const act = actionOf(c.actions)}
		{@const info = actionInfo[act]}
		<li>
			<button
				class="flex w-full items-center gap-3 px-4 py-2 text-left hover:bg-hover"
				onclick={() => (open[c.address] = !open[c.address])}
			>
				<ChevronRight class="size-3.5 shrink-0 text-subtle transition-transform {open[c.address] ? 'rotate-90' : ''}" />
				<span class="grid size-5 shrink-0 place-items-center rounded font-mono text-[12px] font-bold {toneBg[info.tone]}">{info.sym}</span>
				<span class="mono min-w-0 flex-1 truncate text-[12.5px]">{c.address}</span>
				{#if c.action_reason}<span class="hidden text-[11.5px] text-subtle md:inline">{c.action_reason.replaceAll('_', ' ')}</span>{/if}
				<span class="text-[12px] font-medium {toneText[info.tone]}">{info.label}</span>
			</button>
			{#if open[c.address]}
				<div class="border-t border-border bg-surface-2/60 px-4 py-3 pl-[52px]">
					<div class="mb-2 text-[11.5px] text-subtle">{c.provider_name.replace('registry.terraform.io/', '')}</div>
					<table class="mono w-full text-[12px] leading-[1.6]">
						<tbody>
							{#each rows(c) as r (r.key)}
								{#if r.kind !== 'same' || act === 'create' || act === 'delete'}
									<tr class="align-top">
										<td class="w-4 select-none {symTone[r.kind === 'same' ? (act === 'create' ? 'add' : act === 'delete' ? 'remove' : 'same') : r.kind]}">
											{sym[r.kind === 'same' ? (act === 'create' ? 'add' : act === 'delete' ? 'remove' : 'same') : r.kind]}
										</td>
										<td class="w-[30%] pr-4 text-muted">{r.key}</td>
										<td>
											{#if r.kind === 'change'}
												<span class="line-through decoration-danger/60 opacity-70"><ValueView value={r.before} /></span>
												<span class="px-1 text-subtle">→</span>
												<ValueView value={r.after} unknown={r.unknown} />
											{:else if r.kind === 'remove' || act === 'delete'}
												<ValueView value={r.before} />
											{:else}
												<ValueView value={r.after} unknown={r.unknown} />
											{/if}
										</td>
									</tr>
								{/if}
							{/each}
						</tbody>
					</table>
					{#if act === 'update' && rows(c).some((r) => r.kind === 'same')}
						<p class="mt-2 text-[11.5px] text-subtle">{rows(c).filter((r) => r.kind === 'same').length} unchanged attributes hidden</p>
					{/if}
				</div>
			{/if}
		</li>
	{/each}
</ul>

{#if plan.output_changes.filter((o) => actionOf(o.actions) !== 'no-op').length}
	<div class="border-t border-border px-4 py-3">
		<h4 class="mb-2 text-[12px] font-semibold uppercase tracking-wide text-subtle">Outputs</h4>
		<table class="mono w-full text-[12px]">
			<tbody>
				{#each plan.output_changes.filter((o) => actionOf(o.actions) !== 'no-op') as o (o.name)}
					{@const info = actionInfo[actionOf(o.actions)]}
					<tr><td class="w-4 {toneText[info.tone]}">{info.sym}</td><td class="w-[30%] text-muted">{o.name}</td><td><ValueView value={o.after} unknown={o.after_unknown === true} /></td></tr>
				{/each}
			</tbody>
		</table>
	</div>
{/if}
