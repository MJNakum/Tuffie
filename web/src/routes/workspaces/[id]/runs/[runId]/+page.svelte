<script lang="ts">
	import { page } from '$app/state';
	import { onDestroy, untrack } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { api, errMsg, wsUrl } from '#lib/api';
	import { poll } from '#lib/app.svelte';
	import { useWorkspace } from '#lib/workspace.svelte';
	import { ago, dateTime, duration, isActive, kindLabel } from '#lib/format';
	import { ansiToHtml } from '#lib/ansi';
	import type { Plan, PolicyReport, RunDetail, RunEvent, VarOrigin } from '#lib/types';
	import PolicyResults from '#lib/components/PolicyResults.svelte';
	import Badge from '#lib/components/ui/Badge.svelte';
	import ShieldCheck from '@lucide/svelte/icons/shield-check';
	import Variable from '@lucide/svelte/icons/variable';
	import { policyTone, toneBg } from '#lib/format';
	import RunStatus from '#lib/components/RunStatus.svelte';
	import ChangeCounts from '#lib/components/ChangeCounts.svelte';
	import PlanChanges from '#lib/components/PlanChanges.svelte';
	import LogView from '#lib/components/LogView.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Textarea from '#lib/components/ui/Textarea.svelte';
	import Spinner from '#lib/components/ui/Spinner.svelte';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import Check from '@lucide/svelte/icons/check';
	import X from '@lucide/svelte/icons/x';
	import SquareTerminal from '@lucide/svelte/icons/square-terminal';
	import CircleStop from '@lucide/svelte/icons/circle-stop';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';

	const ctx = useWorkspace();
	const runId = $derived(page.params.runId!);

	let run = $state<RunDetail | null>(null);
	let events = $state<RunEvent[]>([]);
	let plan = $state<Plan | null>(null);
	let output = $state<string | null>(null);
	let comment = $state('');
	let busy = $state<'' | 'apply' | 'discard' | 'cancel'>('');
	let planTab = $state<'changes' | 'log'>('changes');
	let showPlanLog = $state(true);
	let showApplyLog = $state(true);
	let sock: WebSocket | null = null;
	let policy = $state<PolicyReport | null>(null);
	let origins = $state<VarOrigin[] | null>(null);
	let showVars = $state(false);

	async function loadRun() {
		try {
			const prev = run?.status;
			run = await api.run(runId);
			if (run.kind === 'cli') {
				if (output === null || isActive(run.status)) output = await api.runOutput(runId).catch(() => '');
			} else if (!plan && run.plan_finished_at) {
				plan = await api.runPlan(runId).catch(() => null);
				if (plan) planTab = 'changes';
			}
			if (!policy && run.policy_status) policy = await api.runPolicy(runId).catch(() => null);
			if (origins === null && run.kind !== 'cli' && run.started_at) origins = await api.runVariables(runId).catch(() => []);
			if (prev && prev !== run.status) ctx.reload();
		} catch (e) {
			toast.error(errMsg(e));
		}
	}

	function connect(id: string) {
		sock?.close();
		events = [];
		sock = new WebSocket(wsUrl(`/runs/${id}/stream`));
		sock.onmessage = (m) => {
			const ev = JSON.parse(m.data);
			if (ev.type === 'eof') return;
			events.push(ev);
			if (ev.type === 'status') loadRun();
		};
	}

	// Only `runId` is a dependency; everything else runs untracked so that
	// state updates from the poll do not restart this effect.
	$effect(() => {
		const id = runId;
		return untrack(() => {
			run = null;
			plan = null;
			output = null;
			policy = null;
			origins = null;
			connect(id);
			return poll(() => {
				if (!run || isActive(run.status) || run.status === 'planned') loadRun();
			}, 1500);
		});
	});
	onDestroy(() => sock?.close());

	// First time a person sees a finished plan: show the run review tour.
	let tourChecked = false;
	$effect(() => {
		if (!tourChecked && plan && run && run.kind !== 'cli') {
			tourChecked = true;
			import('#lib/tour').then((t) => t.autoTour('run', 900));
		}
	});

	const firstApply = $derived(events.find((e) => e.stage === 'apply')?.seq ?? Infinity);
	const planEvents = $derived(
		events.filter((e) => e.seq < firstApply && e.stage !== 'policy' && !(e.type === 'status' && e.data?.status === 'applying'))
	);
	// A drift check shows drifted resources in the same diff view as a plan.
	const driftPlan = $derived<Plan | null>(
		plan && run?.kind === 'drift'
			? { ...plan, resource_changes: plan.drift.map((d) => ({ ...d, name: d.address, module_address: null, action_reason: null, after_unknown: {}, replace_paths: null, importing: null } as any)), output_changes: [] }
			: plan
	);
	const applyEvents = $derived(events.filter((e) => e.seq >= firstApply || (e.stage === 'system' && e.seq > firstApply)));
	const errors = $derived(events.filter((e) => e.type === 'diagnostic' && e.level === 'error'));

	const planState = $derived.by(() => {
		if (!run) return 'pending';
		if (run.status === 'pending') return 'pending';
		if (run.status === 'planning') return 'running';
		if (!run.plan_finished_at) return run.status === 'canceled' ? 'canceled' : 'errored';
		return 'done';
	});
	const applyState = $derived.by(() => {
		if (!run) return 'pending';
		if (run.kind === 'plan_only' || run.kind === 'drift') return 'skipped';
		if (run.status === 'policy_failed') return 'blocked';
		if (run.status === 'planned') return 'needs';
		if (run.status === 'applying') return 'running';
		if (run.status === 'applied') return 'done';
		if (run.status === 'discarded') return 'discarded';
		if (run.status === 'planned_and_finished') return 'skipped';
		if (run.status === 'errored' && run.plan_finished_at && firstApply !== Infinity) return 'errored';
		return 'pending';
	});

	async function act(kind: 'apply' | 'discard' | 'cancel') {
		busy = kind;
		try {
			if (kind === 'apply') await api.applyRun(runId, comment || undefined);
			if (kind === 'discard') await api.discardRun(runId);
			if (kind === 'cancel') await api.cancelRun(runId);
			await loadRun();
			ctx.reload();
		} catch (e) {
			toast.error(errMsg(e));
		} finally {
			busy = '';
		}
	}

	const stepDot: Record<string, string> = {
		pending: 'border-border-strong bg-surface text-subtle',
		running: 'border-primary bg-primary-soft text-primary-text',
		done: 'border-success bg-success-soft text-success',
		needs: 'border-warning bg-warning-soft text-warning',
		errored: 'border-danger bg-danger-soft text-danger',
		canceled: 'border-border-strong bg-surface-3 text-muted',
		discarded: 'border-border-strong bg-surface-3 text-muted',
		skipped: 'border-border bg-surface text-subtle',
		blocked: 'border-danger bg-danger-soft text-danger',
		passed: 'border-success bg-success-soft text-success',
		warned: 'border-warning bg-warning-soft text-warning',
		failed: 'border-danger bg-danger-soft text-danger'
	};
</script>

{#snippet dot(s: string)}
	<span class="absolute -left-[10px] top-3 z-10 grid size-[19px] place-items-center rounded-full border {stepDot[s]}">
		{#if s === 'running'}<span class="size-1.5 rounded-full bg-current pulse-dot"></span>
		{:else if s === 'done'}<Check class="size-3" strokeWidth={3} />
		{:else if s === 'errored' || s === 'blocked' || s === 'failed'}<X class="size-3" strokeWidth={3} />
		{:else if s === 'passed'}<Check class="size-3" strokeWidth={3} />
		{:else if s === 'warned'}<span class="text-[10px] font-bold">!</span>
		{:else if s === 'needs'}<span class="text-[10px] font-bold">!</span>{/if}
	</span>
{/snippet}

<div class="px-8 py-6">
	<nav class="mb-4 flex items-center gap-1 text-[12px] text-subtle">
		<a href="/workspaces/{ctx.id}/runs" class="hover:text-fg">Runs</a>
		<ChevronRight class="size-3" /><span class="mono">{runId}</span>
	</nav>

	{#if !run}
		<div class="grid place-items-center py-16"><Spinner class="size-5" /></div>
	{:else}
		<!-- Summary -->
		<div class="mb-6 flex flex-wrap items-start justify-between gap-4" data-tour="run-summary">
			<div class="min-w-0">
				<div class="flex items-center gap-3">
					<RunStatus status={run.status} kind={run.kind} hasChanges={run.has_changes} />
					{#if run.queue_position}<span class="text-[12px] text-muted">{run.queue_position} run{run.queue_position === 1 ? '' : 's'} ahead in the queue</span>{/if}
				</div>
				<h2 class="mt-2 text-[17px] font-semibold tracking-[-0.01em]">
					{#if run.kind === 'cli'}<span class="mono">{run.command}</span>{:else}{run.message || kindLabel[run.kind]}{/if}
				</h2>
				<div class="mt-1 flex flex-wrap items-center gap-x-3 text-[12px] text-subtle">
					<span class="inline-flex items-center gap-1">{#if run.source === 'terminal'}<SquareTerminal class="size-3" />{/if}{kindLabel[run.kind]}</span>
					<span>·</span><span title={dateTime(run.created_at)}>Queued {ago(run.created_at)}</span>
					{#if run.started_at}<span>·</span><span>Duration {duration(run.started_at, run.finished_at ?? (run.status === 'planned' ? run.plan_finished_at : null))}</span>{/if}
					{#if run.holds_lock}<span>·</span><span class="text-warning">Holds workspace lock</span>{/if}
				</div>
			</div>
			<div class="flex items-center gap-2">
				{#if ['pending', 'planning', 'applying', 'running'].includes(run.status)}
					<Button variant="outline-danger" loading={busy === 'cancel'} onclick={() => act('cancel')}><CircleStop />Cancel run</Button>
				{/if}
			</div>
		</div>

		{#if run.kind === 'cli'}
			<div class="overflow-hidden rounded-lg border border-border">
				<div class="flex h-9 items-center justify-between border-b border-white/5 bg-[var(--term-bg)] px-4 text-[12px] text-[#9aa3b5]">
					<span class="inline-flex items-center gap-2"><SquareTerminal class="size-3.5" />Terminal output</span>
					{#if run.exit_code !== null}<span class="mono {run.exit_code === 0 ? 'text-success' : 'text-danger'}">exit {run.exit_code}</span>{/if}
				</div>
				<pre class="max-h-[70vh] overflow-auto bg-[var(--term-bg)] px-4 py-3 font-mono text-[12px] leading-[1.6] text-[#d7dce6]">{@html ansiToHtml(output ?? '')}</pre>
			</div>
		{:else}
			<!-- Timeline -->
			<ol class="relative ml-[10px] flex flex-col gap-4 border-l border-border pl-0">
				<!-- Plan -->
				<li class="relative -ml-px pl-7" data-tour="plan-step">
					{@render dot(planState)}
					<div class="overflow-hidden rounded-lg border border-border bg-surface">
						<button class="flex w-full items-center gap-3 px-4 py-3 text-left hover:bg-hover" onclick={() => (showPlanLog = !showPlanLog)}>
							<ChevronRight class="size-3.5 text-subtle transition-transform {showPlanLog ? 'rotate-90' : ''}" />
							<span class="text-[13px] font-semibold">{run.kind === 'drift' ? 'Drift check' : 'Plan'}</span>
							<span class="text-[12.5px] text-muted">
								{#if planState === 'pending'}Waiting to start{:else if planState === 'running'}Running…{:else if planState === 'done'}{run.kind === 'drift' ? (run.has_changes ? 'Real infrastructure differs from the state' : 'Infrastructure matches the state') : run.has_changes ? 'Finished with changes' : 'No changes'}{:else if planState === 'canceled'}Canceled{:else}Failed{/if}
							</span>
							<span class="ml-auto flex items-center gap-4">
								{#if run.kind === 'drift' && run.plan_finished_at}
									<span class="text-[12.5px] font-medium {run.has_changes ? 'text-warning' : 'text-success'}">{run.has_changes ? `${Math.max(run.drift_count, 1)} drifted` : 'No drift'}</span>
								{:else if run.plan_finished_at}<ChangeCounts add={run.add_count} change={run.change_count} destroy={run.destroy_count} size="lg" />{/if}
								{#if run.started_at}<span class="text-[12px] text-subtle">{duration(run.started_at, run.plan_finished_at)}</span>{/if}
							</span>
						</button>
						{#if showPlanLog}
							<div class="border-t border-border">
								{#if plan}
									<div class="flex gap-4 border-b border-border px-4">
										{#each [['changes', 'Changes'], ['log', 'Log']] as [k, l] (k)}
											<button
												class="-mb-px border-b-2 py-2 text-[12.5px] font-medium {planTab === k ? 'border-primary text-fg' : 'border-transparent text-muted hover:text-fg'}"
												onclick={() => (planTab = k as typeof planTab)}>{l}</button
											>
										{/each}
									</div>
								{/if}
								{#if driftPlan && planTab === 'changes'}
									{#if plan && plan.resource_drift && run.kind !== 'drift'}
										<div class="flex items-center gap-2 border-b border-border bg-warning-soft px-4 py-2 text-[12.5px] text-warning">
											<TriangleAlert class="size-3.5" />{plan.resource_drift} resource{plan.resource_drift === 1 ? '' : 's'} changed outside Terraform (drift).
										</div>
									{/if}
									<PlanChanges plan={driftPlan} />
								{:else}
									<LogView events={planEvents} />
								{/if}
							</div>
						{/if}
					</div>
				</li>

				<!-- Policy check -->
				{#if run.policy_status}
					<li class="relative -ml-px pl-7" data-tour="policy-step">
						{@render dot(run.policy_status === 'errored' ? 'failed' : run.policy_status)}
						<div class="overflow-hidden rounded-lg border border-border bg-surface">
							<div class="flex items-center gap-3 px-4 py-3">
								<ShieldCheck class="size-4 text-muted" />
								<span class="text-[13px] font-semibold">Policy check</span>
								<span class="inline-flex h-[22px] items-center rounded-full px-2 text-[11.5px] font-medium {toneBg[policyTone[run.policy_status]]}">
									{{ passed: 'Passed', warned: 'Passed with warnings', failed: 'Failed', errored: 'Error' }[run.policy_status]}
								</span>
								{#if policy}<span class="ml-auto text-[12px] text-subtle">{policy.sets.length} policy set{policy.sets.length === 1 ? '' : 's'}</span>{/if}
							</div>
							{#if policy}<div class="border-t border-border"><PolicyResults sets={policy.sets} /></div>{/if}
						</div>
					</li>
				{/if}

				<!-- Apply / confirmation -->
				{#if applyState !== 'skipped'}
					<li class="relative -ml-px pl-7" data-tour="apply-step">
						{@render dot(applyState)}
						{#if applyState === 'blocked'}
							<div class="rounded-lg border border-danger/40 bg-danger-soft px-4 py-3">
								<div class="text-[13px] font-semibold text-danger">Apply blocked by policy</div>
								<p class="mt-0.5 text-[12.5px] text-muted">A mandatory policy set failed. Fix the configuration or the policy, then start a new run.</p>
							</div>
						{:else if applyState === 'needs'}
							<div class="overflow-hidden rounded-lg border border-warning/50 bg-surface">
								<div class="border-b border-border bg-warning-soft px-4 py-3">
									<div class="text-[13px] font-semibold">Confirm and apply</div>
									<p class="mt-0.5 text-[12.5px] text-muted">
										Review the changes above. Apply runs exactly this plan.
										{#if run.destroy_count > 0}<b class="text-danger">{run.destroy_count} resource{run.destroy_count === 1 ? '' : 's'} will be destroyed.</b>{/if}
									</p>
								</div>
								<div class="flex flex-col gap-3 p-4">
									<Textarea bind:value={comment} placeholder="Comment (optional): why are you applying this?" class="min-h-14" />
									<div class="flex justify-end gap-2">
										<Button loading={busy === 'discard'} onclick={() => act('discard')}>Discard run</Button>
										<Button variant={run.kind === 'destroy' ? 'danger' : 'primary'} loading={busy === 'apply'} onclick={() => act('apply')}>
											<Check />{run.kind === 'destroy' ? 'Confirm destroy' : 'Confirm and apply'}
										</Button>
									</div>
								</div>
							</div>
						{:else}
							<div class="overflow-hidden rounded-lg border border-border bg-surface">
								<button class="flex w-full items-center gap-3 px-4 py-3 text-left hover:bg-hover" onclick={() => (showApplyLog = !showApplyLog)}>
									<ChevronRight class="size-3.5 text-subtle transition-transform {showApplyLog ? 'rotate-90' : ''}" />
									<span class="text-[13px] font-semibold">Apply</span>
									<span class="text-[12.5px] text-muted">
										{#if applyState === 'pending'}Waiting for plan{:else if applyState === 'running'}Running…{:else if applyState === 'done'}Applied successfully{:else if applyState === 'discarded'}Discarded{:else}Failed{/if}
									</span>
									{#if run.confirmed_comment}<span class="ml-auto max-w-[40%] truncate text-[12px] italic text-subtle">“{run.confirmed_comment}”</span>{/if}
								</button>
								{#if showApplyLog && (applyState === 'running' || applyState === 'done' || applyState === 'errored')}
									<div class="border-t border-border"><LogView events={applyEvents} /></div>
								{/if}
							</div>
						{/if}
					</li>
				{/if}
			</ol>

			{#if origins && origins.length}
				<div class="mt-5 overflow-hidden rounded-lg border border-border bg-surface" data-tour="run-variables">
					<button class="flex w-full items-center gap-3 px-4 py-3 text-left hover:bg-hover" onclick={() => (showVars = !showVars)}>
						<ChevronRight class="size-3.5 text-subtle transition-transform {showVars ? 'rotate-90' : ''}" />
						<Variable class="size-4 text-muted" />
						<span class="text-[13px] font-semibold">Variables</span>
						<span class="text-[12.5px] text-muted">{origins.filter((o) => !o.overridden).length} used · where each value came from</span>
					</button>
					{#if showVars}
						<table class="w-full border-t border-border text-[12.5px]">
							<thead class="text-[11px] uppercase tracking-wide text-subtle">
								<tr class="border-b border-border"><th class="px-4 py-2 text-left font-medium">Key</th><th class="px-4 py-2 text-left font-medium">Type</th><th class="px-4 py-2 text-left font-medium">Source</th><th class="px-4 py-2 text-left font-medium">Result</th></tr>
							</thead>
							<tbody class="divide-y divide-border">
								{#each origins as o, i (i)}
									<tr class={o.overridden ? 'text-subtle' : ''}>
										<td class="mono px-4 py-2 {o.overridden ? 'line-through' : ''}">{o.key}{#if o.sensitive}<Badge class="ml-2">sensitive</Badge>{/if}</td>
										<td class="px-4 py-2">{o.category === 'env' ? 'Environment' : 'Terraform'}</td>
										<td class="px-4 py-2">{#if o.set_id}<a href="/variable-sets/{o.set_id}" class="hover:text-primary-text">Set: {o.set_name}</a>{:else}Workspace{/if}</td>
										<td class="px-4 py-2">{#if o.overridden}Overridden by {o.overridden_by}{:else}<span class="text-success">Used</span>{/if}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					{/if}
				</div>
			{/if}

			{#if errors.length}
				<div class="mt-5 rounded-lg border border-danger/40 bg-danger-soft p-4">
					<div class="mb-2 flex items-center gap-2 text-[13px] font-semibold text-danger"><TriangleAlert class="size-4" />Errors</div>
					{#each errors as e (e.seq)}
						<div class="mb-2 last:mb-0">
							<div class="text-[13px] text-fg">{e.data?.diagnostic?.summary ?? e.message}</div>
							{#if e.data?.diagnostic?.detail}<div class="mt-0.5 whitespace-pre-wrap text-[12.5px] text-muted">{e.data.diagnostic.detail}</div>{/if}
							{#if e.data?.diagnostic?.range}<div class="mono mt-0.5 text-[11.5px] text-subtle">{e.data.diagnostic.range.filename}:{e.data.diagnostic.range.start.line}</div>{/if}
						</div>
					{/each}
				</div>
			{/if}
		{/if}
	{/if}
</div>
