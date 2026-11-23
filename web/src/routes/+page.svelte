<script lang="ts">
	import { api, errMsg } from '#lib/api';
	import { poll } from '#lib/app.svelte';
	import { ago, shortPath } from '#lib/format';
	import type { WorkspaceSummary } from '#lib/types';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import RunStatus from '#lib/components/RunStatus.svelte';
	import ChangeCounts from '#lib/components/ChangeCounts.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Input from '#lib/components/ui/Input.svelte';
	import Empty from '#lib/components/ui/Empty.svelte';
	import Spinner from '#lib/components/ui/Spinner.svelte';
	import Plus from '@lucide/svelte/icons/plus';
	import Search from '@lucide/svelte/icons/search';
	import Folder from '@lucide/svelte/icons/folder';
	import GitBranch from '@lucide/svelte/icons/git-branch';
	import Boxes from '@lucide/svelte/icons/boxes';
	import Lock from '@lucide/svelte/icons/lock';
	import Radar from '@lucide/svelte/icons/radar';

	let list = $state<WorkspaceSummary[] | null>(null);
	let error = $state('');
	let q = $state('');

	$effect(() =>
		poll(async () => {
			try {
				list = await api.workspaces();
				error = '';
			} catch (e) {
				error = errMsg(e);
			}
		}, 4000)
	);

	const shown = $derived(
		(list ?? []).filter((w) => !q || w.name.toLowerCase().includes(q.toLowerCase()) || w.description.toLowerCase().includes(q.toLowerCase()))
	);
	const needsAttention = $derived((list ?? []).filter((w) => w.latest_run?.status === 'planned').length);
</script>

<PageHeader title="Workspaces" subtitle="Each workspace is one Terraform or OpenTofu configuration with its own runs, state and variables.">
	{#snippet actions()}
		<Button variant="primary" href="/workspaces/new"><Plus />New workspace</Button>
	{/snippet}
</PageHeader>

<div class="min-h-0 flex-1 overflow-auto px-8 py-6">
	{#if error}
		<div class="mb-4 rounded-md border border-danger/40 bg-danger-soft px-3 py-2 text-[13px] text-danger">{error}</div>
	{/if}

	{#if list === null}
		<div class="grid place-items-center py-20"><Spinner class="size-5" /></div>
	{:else if list.length === 0}
		<div class="rounded-lg border border-dashed border-border-strong bg-surface">
			<Empty
				icon={Boxes}
				title="Create your first workspace"
				text="Point Tuffie at a folder with .tf files, or at a Git repository. You can then plan, review and apply from the UI or the terminal."
			>
				<Button variant="primary" href="/workspaces/new"><Plus />New workspace</Button>
			</Empty>
		</div>
	{:else}
		<div class="mb-4 flex items-center gap-3">
			<div class="relative w-72">
				<Search class="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-subtle" />
				<Input bind:value={q} placeholder="Search workspaces" class="pl-8" />
			</div>
			<span class="text-[12.5px] text-subtle">{list.length} workspace{list.length === 1 ? '' : 's'}</span>
			{#if needsAttention}
				<span class="ml-auto inline-flex items-center gap-1.5 rounded-full bg-warning-soft px-2.5 py-0.5 text-[12px] font-medium text-warning">
					{needsAttention} run{needsAttention === 1 ? '' : 's'} waiting for confirmation
				</span>
			{/if}
		</div>

		<div class="overflow-hidden rounded-lg border border-border bg-surface">
			<table class="w-full text-left text-[13px]">
				<thead class="border-b border-border bg-surface-2 text-[11.5px] font-medium uppercase tracking-wide text-subtle">
					<tr>
						<th class="px-4 py-2.5 font-medium">Name</th>
						<th class="px-4 py-2.5 font-medium">Latest run</th>
						<th class="px-4 py-2.5 font-medium">Changes</th>
						<th class="px-4 py-2.5 text-right font-medium">Resources</th>
						<th class="px-4 py-2.5 font-medium">Source</th>
						<th class="px-4 py-2.5 font-medium">Tool</th>
						<th class="px-4 py-2.5 text-right font-medium">Updated</th>
					</tr>
				</thead>
				<tbody class="divide-y divide-border">
					{#each shown as w (w.id)}
						<tr class="group relative transition-colors hover:bg-hover">
							<td class="px-4 py-3">
								<a href="/workspaces/{w.id}" class="flex items-center gap-2 font-medium text-fg after:absolute after:inset-0">
									{w.name}
									{#if w.lock_run_id}<Lock class="size-3 text-warning" />{/if}
									{#if w.drift_status === 'drifted'}
										<span class="relative z-10 inline-flex h-[18px] items-center gap-1 rounded-full bg-warning-soft px-1.5 text-[11px] font-medium text-warning" title="{w.drift_count} resource(s) changed outside Terraform"><Radar class="size-3" />Drifted</span>
									{/if}
								</a>
								{#if w.description}<div class="mt-0.5 max-w-[280px] truncate text-[12px] text-subtle">{w.description}</div>{/if}
							</td>
							<td class="px-4 py-3">
								{#if w.latest_run}<RunStatus status={w.latest_run.status} />{:else}<span class="text-subtle">No runs</span>{/if}
							</td>
							<td class="px-4 py-3">
								{#if w.latest_run && w.latest_run.kind !== 'cli' && w.latest_run.has_changes !== null}
									<ChangeCounts add={w.latest_run.add_count} change={w.latest_run.change_count} destroy={w.latest_run.destroy_count} />
								{:else}<span class="text-subtle">—</span>{/if}
							</td>
							<td class="px-4 py-3 text-right tabular text-muted">{w.resource_count ?? '—'}</td>
							<td class="px-4 py-3">
								<span class="inline-flex max-w-[260px] items-center gap-1.5 text-[12px] text-muted" title={w.source_kind === 'git' ? w.git_url : w.local_path}>
									{#if w.source_kind === 'git'}<GitBranch class="size-3.5 shrink-0" />{:else}<Folder class="size-3.5 shrink-0" />{/if}
									<span class="mono truncate">{w.source_kind === 'git' ? shortPath(w.git_url?.replace(/^https?:\/\//, ''), 36) : shortPath(w.local_path, 36)}</span>
								</span>
							</td>
							<td class="px-4 py-3"><span class="mono text-[12px] text-muted">{w.tool}</span></td>
							<td class="px-4 py-3 text-right text-[12px] text-subtle">{ago(w.latest_run?.created_at ?? w.updated_at)}</td>
						</tr>
					{:else}
						<tr><td colspan="7" class="px-4 py-10 text-center text-muted">No workspace matches “{q}”.</td></tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>
