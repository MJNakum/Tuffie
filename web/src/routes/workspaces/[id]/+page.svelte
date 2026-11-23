<script lang="ts">
	import { api } from '#lib/api';
	import { poll } from '#lib/app.svelte';
	import { useWorkspace } from '#lib/workspace.svelte';
	import { ago, dateTime, kindLabel } from '#lib/format';
	import type { Run, StateView } from '#lib/types';
	import Card from '#lib/components/ui/Card.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Spinner from '#lib/components/ui/Spinner.svelte';
	import RunStatus from '#lib/components/RunStatus.svelte';
	import ChangeCounts from '#lib/components/ChangeCounts.svelte';
	import ValueView from '#lib/components/ValueView.svelte';
	import ArrowRight from '@lucide/svelte/icons/arrow-right';
	import SquareTerminal from '@lucide/svelte/icons/square-terminal';
	import Radar from '@lucide/svelte/icons/radar';
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import { errMsg } from '#lib/api';
	import { intervalLabel } from '#lib/format';

	const ctx = useWorkspace();
	const ws = $derived(ctx.ws!);
	let runs = $state<Run[] | null>(null);
	let st = $state<StateView | null>(null);
	let stErr = $state('');

	$effect(() => poll(async () => (runs = await api.runs(ctx.id, 15).catch(() => runs)), 3000));
	$effect(() => {
		ctx.version;
		api.state(ctx.id).then(
			(s) => ((st = s), (stErr = '')),
			(e) => (stErr = e.message)
		);
	});

	// Drift checks and terminal commands do not count as the latest run.
	const latest = $derived(runs?.find((r) => r.kind !== 'cli' && r.kind !== 'drift') ?? null);
	const byType = $derived.by(() => {
		const m = new Map<string, number>();
		for (const r of st?.resources ?? []) if (r.mode === 'managed') m.set(r.type, (m.get(r.type) ?? 0) + 1);
		return [...m].sort((a, b) => b[1] - a[1]);
	});
	const managed = $derived((st?.resources ?? []).filter((r) => r.mode === 'managed').length);

	let checking = $state(false);
	async function checkDrift() {
		checking = true;
		try {
			const r = await api.checkDrift(ctx.id);
			goto(`/workspaces/${ctx.id}/runs/${r.id}`);
		} catch (e) {
			toast.error(errMsg(e));
		} finally {
			checking = false;
		}
	}
	const nextCheck = $derived.by(() => {
		if (!ws.drift_interval_minutes) return null;
		if (!ws.drift_checked_at) return 'within a minute';
		const t = new Date(ws.drift_checked_at).getTime() + ws.drift_interval_minutes * 60000 - Date.now();
		if (t <= 60000) return 'within a minute';
		const m = Math.round(t / 60000);
		return m < 120 ? `in ${m} min` : `in ${Math.round(m / 60)} h`;
	});
</script>

<div class="grid gap-5 px-8 py-6 xl:grid-cols-[minmax(0,1fr)_340px]">
	<div class="flex min-w-0 flex-col gap-5">
		<Card title="Latest run" pad={false}>
			{#snippet actions()}<Button size="sm" variant="ghost" href="/workspaces/{ws.id}/runs">All runs<ArrowRight /></Button>{/snippet}
			{#if runs === null}
				<div class="grid place-items-center py-10"><Spinner /></div>
			{:else if !latest}
				<div class="px-4 py-8 text-center text-[13px] text-muted">No runs yet. Use <b class="text-fg">Start run</b> to create the first plan.</div>
			{:else}
				<a href="/workspaces/{ws.id}/runs/{latest.id}" class="flex items-center gap-4 px-4 py-4 hover:bg-hover">
					<RunStatus status={latest.status} />
					<div class="min-w-0 flex-1">
						<div class="truncate text-[13px] font-medium">{latest.message || kindLabel[latest.kind]}</div>
						<div class="mt-0.5 text-[12px] text-subtle">{kindLabel[latest.kind]} · <span class="mono">{latest.id}</span> · {ago(latest.created_at)}</div>
					</div>
					{#if latest.has_changes !== null}
						<ChangeCounts add={latest.add_count} change={latest.change_count} destroy={latest.destroy_count} size="lg" />
					{/if}
					<ArrowRight class="size-4 text-subtle" />
				</a>
				{#if latest.status === 'planned'}
					<div class="flex items-center justify-between border-t border-border bg-warning-soft px-4 py-2.5 text-[12.5px]">
						<span class="text-warning">This plan waits for your confirmation. The workspace is locked until you apply or discard it.</span>
						<Button size="sm" variant="primary" href="/workspaces/{ws.id}/runs/{latest.id}">Review</Button>
					</div>
				{/if}
			{/if}
			{#if runs && runs.length > 1}
				<ul class="divide-y divide-border border-t border-border">
					{#each runs.filter((r) => r.id !== latest?.id).slice(0, 4) as r (r.id)}
						<li>
							<a href="/workspaces/{ws.id}/runs/{r.id}" class="flex items-center gap-3 px-4 py-2 text-[12.5px] hover:bg-hover">
								<RunStatus status={r.status} variant="icon" kind={r.kind} hasChanges={r.has_changes} />
								{#if r.kind === 'cli'}
									<SquareTerminal class="size-3.5 text-subtle" /><span class="mono min-w-0 flex-1 truncate text-muted">{r.command}</span>
								{:else}
									<span class="min-w-0 flex-1 truncate text-muted">{r.message || kindLabel[r.kind]}</span>
								{/if}
								<span class="text-subtle">{ago(r.created_at)}</span>
							</a>
						</li>
					{/each}
				</ul>
			{/if}
		</Card>

		<Card title="Resources" pad={false}>
			{#snippet actions()}<Button size="sm" variant="ghost" href="/workspaces/{ws.id}/state">Open state<ArrowRight /></Button>{/snippet}
			{#if stErr}
				<p class="px-4 py-6 text-[13px] text-muted">{stErr}</p>
			{:else if st === null}
				<div class="grid place-items-center py-10"><Spinner /></div>
			{:else if !st.initialized}
				<p class="px-4 py-6 text-[13px] text-muted">The workspace is not initialized yet. The first run runs <code>init</code> for you.</p>
			{:else if managed === 0}
				<p class="px-4 py-6 text-[13px] text-muted">The state has no resources.</p>
			{:else}
				<div class="flex items-baseline gap-2 px-4 pt-4">
					<span class="text-[28px] font-semibold tracking-[-0.02em] tabular">{managed}</span>
					<span class="text-[13px] text-muted">managed resources · {byType.length} types</span>
				</div>
				<ul class="grid gap-x-6 px-4 pb-4 pt-3 sm:grid-cols-2">
					{#each byType.slice(0, 10) as [t, n] (t)}
						<li class="flex items-center justify-between border-b border-border py-1.5 text-[12.5px]">
							<span class="mono truncate text-muted">{t}</span><span class="tabular">{n}</span>
						</li>
					{/each}
				</ul>
			{/if}
		</Card>
	</div>

	<div class="flex flex-col gap-5">
		<Card title="Drift">
			{#snippet actions()}<Button size="sm" variant="ghost" loading={checking} onclick={checkDrift}><Radar />Check now</Button>{/snippet}
			<div class="flex items-start gap-3">
				<span
					class="mt-0.5 grid size-8 shrink-0 place-items-center rounded-lg {ws.drift_status === 'drifted'
						? 'bg-warning-soft text-warning'
						: ws.drift_status === 'ok'
							? 'bg-success-soft text-success'
							: ws.drift_status === 'error'
								? 'bg-danger-soft text-danger'
								: 'bg-surface-3 text-subtle'}"><Radar class="size-4" /></span
				>
				<div class="min-w-0 text-[12.5px]">
					<div class="text-[13px] font-medium">
						{#if ws.drift_status === 'drifted'}{ws.drift_count} resource{ws.drift_count === 1 ? '' : 's'} drifted
						{:else if ws.drift_status === 'ok'}No drift
						{:else if ws.drift_status === 'error'}Last check failed
						{:else}Not checked yet{/if}
					</div>
					<div class="mt-0.5 text-muted">
						{#if ws.drift_checked_at}Checked {ago(ws.drift_checked_at)}{#if ws.drift_run_id} · <a class="text-primary-text hover:underline" href="/workspaces/{ws.id}/runs/{ws.drift_run_id}">details</a>{/if}{:else}Compares real infrastructure with the state.{/if}
					</div>
					<div class="mt-1 text-subtle">
						{#if ws.drift_interval_minutes}Schedule: {intervalLabel(ws.drift_interval_minutes).toLowerCase()} · next {nextCheck}{:else}No schedule. <a class="text-primary-text hover:underline" href="/workspaces/{ws.id}/settings">Set one</a>{/if}
					</div>
				</div>
			</div>
		</Card>
		<Card title="Details">
			<dl class="grid grid-cols-[110px_minmax(0,1fr)] gap-x-3 gap-y-2.5 text-[12.5px]">
				<dt class="text-subtle">ID</dt><dd class="mono truncate">{ws.id}</dd>
				<dt class="text-subtle">Source</dt><dd>{ws.source_kind === 'git' ? 'Git repository' : 'Local folder'}</dd>
				{#if ws.source_kind === 'git'}
					<dt class="text-subtle">Repository</dt><dd class="mono break-all">{ws.git_url}</dd>
					<dt class="text-subtle">Ref</dt><dd class="mono">{ws.git_ref || 'default branch'}</dd>
					{#if ws.commit}<dt class="text-subtle">Commit</dt><dd><span class="mono">{ws.commit.sha.slice(0, 8)}</span> <span class="text-muted">{ws.commit.subject}</span></dd>{/if}
				{/if}
				<dt class="text-subtle">Folder</dt><dd class="mono break-all">{ws.resolved_dir}</dd>
				<dt class="text-subtle">Tool</dt><dd class="mono">{ws.tool} {ws.tool_version ?? ''}</dd>
				<dt class="text-subtle">Initialized</dt><dd>{ws.initialized ? 'Yes' : 'No'}</dd>
				<dt class="text-subtle">Auto-apply</dt><dd>{ws.auto_apply ? 'On' : 'Off'}</dd>
				<dt class="text-subtle">Created</dt><dd>{dateTime(ws.created_at)}</dd>
			</dl>
		</Card>
		{#if st?.outputs?.length}
			<Card title="Outputs">
				<dl class="flex flex-col gap-2.5 text-[12.5px]">
					{#each st.outputs as o (o.name)}
						<div><dt class="mono text-subtle">{o.name}</dt><dd class="mono mt-0.5"><ValueView value={o.value} /></dd></div>
					{/each}
				</dl>
			</Card>
		{/if}
	</div>
</div>
