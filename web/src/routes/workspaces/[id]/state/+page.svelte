<script lang="ts">
	import { api, errMsg } from '#lib/api';
	import { useWorkspace } from '#lib/workspace.svelte';
	import { ago, kindLabel } from '#lib/format';
	import type { StateView, StateVersion } from '#lib/types';
	import ResourceGraph from '#lib/components/ResourceGraph.svelte';
	import ValueView from '#lib/components/ValueView.svelte';
	import Input from '#lib/components/ui/Input.svelte';
	import Spinner from '#lib/components/ui/Spinner.svelte';
	import Empty from '#lib/components/ui/Empty.svelte';
	import Select from '#lib/components/ui/Select.svelte';
	import Database from '@lucide/svelte/icons/database';
	import Search from '@lucide/svelte/icons/search';
	import Copy from '@lucide/svelte/icons/copy';

	const ctx = useWorkspace();
	let versions = $state<StateVersion[]>([]);
	let source = $state('current');
	let st = $state<StateView | null>(null);
	let error = $state('');
	let loading = $state(true);
	let view = $state<'resources' | 'graph' | 'outputs'>('resources');
	let q = $state('');
	let selected = $state<string | null>(null);

	$effect(() => {
		ctx.version;
		api.stateVersions(ctx.id).then((v) => (versions = v)).catch(() => {});
	});
	$effect(() => {
		const s = source;
		loading = true;
		error = '';
		(s === 'current' ? api.state(ctx.id) : api.runState(s))
			.then((v) => {
				st = v;
				if (!selected || !v.resources.find((r) => r.address === selected)) selected = v.resources[0]?.address ?? null;
			})
			.catch((e) => (error = errMsg(e)))
			.finally(() => (loading = false));
	});

	const shown = $derived((st?.resources ?? []).filter((r) => !q || r.address.toLowerCase().includes(q.toLowerCase())));
	const groups = $derived.by(() => {
		const m = new Map<string, typeof shown>();
		for (const r of shown) {
			const k = r.module || 'root module';
			if (!m.has(k)) m.set(k, []);
			m.get(k)!.push(r);
		}
		return [...m];
	});
	const current = $derived(st?.resources.find((r) => r.address === selected) ?? null);
	const options = $derived([
		{ value: 'current', label: 'Current state (live)' },
		...versions.map((v) => ({ value: v.run_id, label: `Serial ${v.serial ?? '?'} · after ${kindLabel[v.kind].toLowerCase()} · ${ago(v.created_at)}` }))
	]);
</script>

<div class="flex h-full flex-col px-8 py-6">
	<div class="mb-4 flex flex-wrap items-center gap-3">
		<div class="flex items-center gap-1 rounded-md bg-surface-2 p-0.5">
			{#each [['resources', 'Resources'], ['graph', 'Graph'], ['outputs', 'Outputs']] as [k, l] (k)}
				<button
					class="h-7 rounded px-3 text-[12.5px] font-medium {view === k ? 'bg-surface-3 text-fg shadow-sm' : 'text-muted hover:text-fg'}"
					onclick={() => (view = k as typeof view)}>{l}</button
				>
			{/each}
		</div>
		<Select bind:value={source} {options} class="w-80" />
		{#if st}<span class="text-[12.5px] text-subtle">{st.resources.length} resources{st.terraform_version ? ` · written by ${st.terraform_version}` : ''}</span>{/if}
	</div>

	{#if loading && !st}
		<div class="grid place-items-center py-16"><Spinner /></div>
	{:else if error}
		<div class="rounded-md border border-danger/40 bg-danger-soft px-3 py-2 text-[13px] text-danger">{error}</div>
	{:else if st && (st.initialized === false || st.resources.length === 0)}
		<div class="rounded-lg border border-dashed border-border-strong bg-surface">
			<Empty icon={Database} title="No resources in state" text={st.initialized === false ? 'Run a plan first. Tuffie runs init for you.' : 'Apply a run to create resources.'} />
		</div>
	{:else if st}
		{#if view === 'resources'}
			<div class="grid min-h-[480px] flex-1 grid-cols-[minmax(260px,360px)_minmax(0,1fr)] overflow-hidden rounded-lg border border-border bg-surface">
				<div class="flex min-h-0 flex-col border-r border-border">
					<div class="relative border-b border-border p-2">
						<Search class="pointer-events-none absolute left-4.5 top-1/2 size-3.5 -translate-y-1/2 text-subtle" />
						<Input bind:value={q} placeholder="Filter resources" class="h-7 pl-7" />
					</div>
					<div class="min-h-0 flex-1 overflow-auto py-1">
						{#each groups as [mod, rs] (mod)}
							<div class="px-3 pb-1 pt-2 text-[11px] font-medium uppercase tracking-wide text-subtle">{mod}</div>
							{#each rs as r (r.address)}
								<button
									onclick={() => (selected = r.address)}
									class="flex w-full items-center gap-2 px-3 py-1.5 text-left {selected === r.address ? 'bg-primary-soft' : 'hover:bg-hover'}"
								>
									<span class="size-1.5 shrink-0 rounded-full {r.mode === 'data' ? 'bg-violet' : 'bg-primary'}"></span>
									<span class="mono min-w-0 flex-1 truncate text-[12px] {selected === r.address ? 'text-primary-text' : ''}">{r.address}</span>
									{#if r.tainted}<span class="text-[10.5px] text-warning">tainted</span>{/if}
								</button>
							{/each}
						{/each}
					</div>
				</div>
				<div class="min-h-0 overflow-auto">
					{#if current}
						<div class="sticky top-0 flex items-start justify-between gap-4 border-b border-border bg-surface px-5 py-3">
							<div class="min-w-0">
								<div class="mono truncate text-[13.5px] font-medium">{current.address}</div>
								<div class="mt-0.5 text-[12px] text-subtle">{current.mode === 'data' ? 'Data source' : 'Managed resource'} · {current.provider_name.replace(/^provider\[|"|\]$/g, '').replace('registry.terraform.io/', '')}</div>
							</div>
							<button class="grid size-7 place-items-center rounded-md text-subtle hover:bg-hover hover:text-fg" title="Copy address" onclick={() => navigator.clipboard.writeText(current.address)}><Copy class="size-3.5" /></button>
						</div>
						<table class="mono w-full text-[12px] leading-[1.6]">
							<tbody>
								{#each Object.entries(current.values ?? {}) as [k, v] (k)}
									<tr class="border-b border-border align-top">
										<td class="w-[32%] px-5 py-1.5 text-muted">{k}</td>
										<td class="px-5 py-1.5"><ValueView value={v} /></td>
									</tr>
								{/each}
							</tbody>
						</table>
						{#if current.depends_on?.length}
							<div class="px-5 py-4">
								<div class="mb-2 text-[11.5px] font-medium uppercase tracking-wide text-subtle">Depends on</div>
								<div class="flex flex-wrap gap-1.5">
									{#each current.depends_on as d (d)}
										<button class="mono rounded border border-border-strong px-1.5 py-0.5 text-[11.5px] hover:border-primary hover:text-primary-text" onclick={() => (selected = st?.resources.find((r) => r.address === d || r.address.startsWith(d + '['))?.address ?? selected)}>{d}</button>
									{/each}
								</div>
							</div>
						{/if}
					{/if}
				</div>
			</div>
		{:else if view === 'graph'}
			<div class="min-h-[480px] flex-1 overflow-hidden rounded-lg border border-border bg-surface">
				<ResourceGraph graph={st.graph} {selected} onSelect={(id) => (selected = selected === id ? null : id)} />
			</div>
			<p class="mt-2 text-[12px] text-subtle">Arrows point from a dependency to the resource that uses it. Click a resource to highlight its links.</p>
		{:else}
			<div class="overflow-hidden rounded-lg border border-border bg-surface">
				{#if st.outputs.length === 0}
					<p class="px-4 py-8 text-center text-[13px] text-muted">No outputs.</p>
				{/if}
				<table class="w-full text-[12.5px]">
					<tbody class="divide-y divide-border">
						{#each st.outputs as o (o.name)}
							<tr class="align-top"><td class="mono w-[30%] px-4 py-2.5 text-muted">{o.name}</td><td class="mono px-4 py-2.5"><ValueView value={o.value} /></td></tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	{/if}
</div>
