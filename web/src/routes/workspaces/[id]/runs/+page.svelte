<script lang="ts">
	import { api } from '#lib/api';
	import { poll } from '#lib/app.svelte';
	import { useWorkspace } from '#lib/workspace.svelte';
	import { ago, duration, kindLabel } from '#lib/format';
	import type { Run } from '#lib/types';
	import RunStatus from '#lib/components/RunStatus.svelte';
	import ChangeCounts from '#lib/components/ChangeCounts.svelte';
	import Spinner from '#lib/components/ui/Spinner.svelte';
	import Empty from '#lib/components/ui/Empty.svelte';
	import SquareTerminal from '@lucide/svelte/icons/square-terminal';
	import MousePointerClick from '@lucide/svelte/icons/mouse-pointer-click';
	import History from '@lucide/svelte/icons/history';

	const ctx = useWorkspace();
	let runs = $state<Run[] | null>(null);
	let filter = $state<'all' | 'ui' | 'terminal'>('all');
	$effect(() => poll(async () => (runs = await api.runs(ctx.id, 200).catch(() => runs)), 2500));
	const shown = $derived((runs ?? []).filter((r) => filter === 'all' || r.source === filter));
</script>

<div class="px-8 py-6">
	<div class="mb-4 flex items-center gap-1 rounded-md bg-surface-2 p-0.5 w-fit">
		{#each [['all', 'All'], ['ui', 'UI runs'], ['terminal', 'Terminal']] as [k, l] (k)}
			<button
				class="h-7 rounded px-3 text-[12.5px] font-medium {filter === k ? 'bg-surface-3 text-fg shadow-sm' : 'text-muted hover:text-fg'}"
				onclick={() => (filter = k as typeof filter)}>{l}</button
			>
		{/each}
	</div>

	{#if runs === null}
		<div class="grid place-items-center py-16"><Spinner /></div>
	{:else if shown.length === 0}
		<div class="rounded-lg border border-dashed border-border-strong bg-surface">
			<Empty icon={History} title="No runs yet" text="Start a run from the button above, or type a command in the terminal. Both appear here." />
		</div>
	{:else}
		<div class="overflow-hidden rounded-lg border border-border bg-surface">
			<ul class="divide-y divide-border">
				{#each shown as r (r.id)}
					<li>
						<a href="/workspaces/{ctx.id}/runs/{r.id}" class="grid grid-cols-[22px_minmax(0,1fr)_auto_auto_90px] items-center gap-4 px-4 py-3 hover:bg-hover">
							<RunStatus status={r.status} variant="icon" />
							<div class="min-w-0">
								{#if r.kind === 'cli'}
									<div class="mono truncate text-[12.5px]">{r.command}</div>
								{:else}
									<div class="truncate text-[13px] font-medium">{r.message || kindLabel[r.kind]}</div>
								{/if}
								<div class="mt-0.5 flex items-center gap-2 text-[12px] text-subtle">
									<span class="inline-flex items-center gap-1">
										{#if r.source === 'terminal'}<SquareTerminal class="size-3" />Terminal{:else}<MousePointerClick class="size-3" />{kindLabel[r.kind]}{/if}
									</span>
									<span>·</span><span class="mono">{r.id}</span>
									{#if r.started_at}<span>·</span><span>{duration(r.started_at, r.finished_at)}</span>{/if}
								</div>
							</div>
							<div>
								{#if r.kind !== 'cli' && r.has_changes !== null}<ChangeCounts add={r.add_count} change={r.change_count} destroy={r.destroy_count} />{/if}
							</div>
							<RunStatus status={r.status} variant="text" />
							<span class="text-right text-[12px] text-subtle">{ago(r.created_at)}</span>
						</a>
					</li>
				{/each}
			</ul>
		</div>
	{/if}
</div>
