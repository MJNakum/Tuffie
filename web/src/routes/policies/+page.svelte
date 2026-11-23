<script lang="ts">
	import ListSkeleton from '#lib/components/ui/ListSkeleton.svelte';
	import { goto } from '$app/navigation';
	import { api, errMsg } from '#lib/api';
	import { ago } from '#lib/format';
	import type { PolicySet } from '#lib/types';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import ScopePicker from '#lib/components/ScopePicker.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Dialog from '#lib/components/ui/Dialog.svelte';
	import Field from '#lib/components/ui/Field.svelte';
	import Input from '#lib/components/ui/Input.svelte';
	import Textarea from '#lib/components/ui/Textarea.svelte';
	import Badge from '#lib/components/ui/Badge.svelte';
	import Spinner from '#lib/components/ui/Spinner.svelte';
	import Empty from '#lib/components/ui/Empty.svelte';
	import Plus from '@lucide/svelte/icons/plus';
	import ShieldCheck from '@lucide/svelte/icons/shield-check';
	import Check from '@lucide/svelte/icons/check';

	let sets = $state<PolicySet[] | null>(null);
	let open = $state(false);
	const blank = () => ({
		name: '',
		description: '',
		enforcement: 'advisory' as 'advisory' | 'mandatory',
		source_kind: 'inline' as 'inline' | 'path',
		source_path: '',
		global: false,
		workspace_ids: [] as string[]
	});
	let f = $state(blank());
	let saving = $state(false);
	let err = $state('');

	$effect(() => {
		api.policySets().then((s) => (sets = s));
	});

	async function create() {
		saving = true;
		err = '';
		try {
			const s = await api.createPolicySet({ ...f, source_path: f.source_kind === 'path' ? f.source_path : null });
			goto(`/policies/${s.id}`);
		} catch (e) {
			err = errMsg(e);
		} finally {
			saving = false;
		}
	}
</script>

<PageHeader title="Policies" subtitle="Check every plan against rules written in Rego (Open Policy Agent), before anything is applied.">
	{#snippet actions()}
		<Button variant="primary" onclick={() => ((f = blank()), (err = ''), (open = true))}><Plus />New policy set</Button>
	{/snippet}
</PageHeader>

<div class="min-h-0 flex-1 overflow-auto px-8 py-6">
	{#if sets === null}
		<ListSkeleton rows={3} />
	{:else if sets.length === 0}
		<div class="motion-rise rounded-lg border border-dashed border-border-strong bg-surface">
			<Empty icon={ShieldCheck} title="No policy sets" text="A policy set holds Rego policies. Tuffie evaluates them against each plan. A mandatory set that fails blocks the apply. Conftest-style policies (deny, warn, violation) work as they are." />
		</div>
	{:else}
		<div class="overflow-hidden rounded-lg border border-border bg-surface">
			<table class="w-full text-left text-base">
				<thead class="border-b border-border bg-surface-2 text-2xs uppercase tracking-wide text-subtle">
					<tr><th class="px-4 py-2.5 font-medium">Name</th><th class="px-4 py-2.5 font-medium">Enforcement</th><th class="px-4 py-2.5 font-medium">Source</th><th class="px-4 py-2.5 font-medium">Applies to</th><th class="px-4 py-2.5 text-right font-medium">Updated</th></tr>
				</thead>
				<tbody class="divide-y divide-border">
					{#each sets as s (s.id)}
						<tr class="relative hover:bg-hover">
							<td class="px-4 py-3">
								<a href="/policies/{s.id}" class="font-medium after:absolute after:inset-0">{s.name}</a>
								{#if s.description}<div class="mt-0.5 text-xs text-subtle">{s.description}</div>{/if}
							</td>
							<td class="px-4 py-3"><Badge tone={s.enforcement === 'mandatory' ? 'danger' : 'neutral'}>{s.enforcement}</Badge></td>
							<td class="px-4 py-3 text-sm text-muted">
								{#if s.source_kind === 'path'}<span class="mono">{s.source_path}</span>{#if s.load_error}<span class="ml-1 text-danger">(error)</span>{/if}{:else}{s.policies.length} polic{s.policies.length === 1 ? 'y' : 'ies'}{/if}
							</td>
							<td class="px-4 py-3">{#if s.global}<Badge tone="primary">All workspaces</Badge>{:else}<span class="text-muted">{s.workspace_ids.length} workspace{s.workspace_ids.length === 1 ? '' : 's'}</span>{/if}</td>
							<td class="px-4 py-3 text-right text-xs text-subtle">{ago(s.updated_at)}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>

<Dialog bind:open title="New policy set" width="max-w-xl">
	<form id="psform" class="flex flex-col gap-4" onsubmit={(e) => (e.preventDefault(), create())}>
		<Field label="Name" for="n"><Input id="n" bind:value={f.name} placeholder="security-baseline" required /></Field>
		<Field label="Description" for="d"><Textarea id="d" bind:value={f.description} class="min-h-14" placeholder="Optional" /></Field>
		<div class="flex flex-col gap-1.5">
			<span class="text-sm font-medium">Enforcement</span>
			<div class="grid grid-cols-2 gap-3">
				{#each [{ k: 'advisory', t: 'Advisory', d: 'Failures show as warnings. Apply is allowed.' }, { k: 'mandatory', t: 'Mandatory', d: 'A failure blocks the apply.' }] as o (o.k)}
					<button type="button" onclick={() => (f.enforcement = o.k as typeof f.enforcement)} class="relative rounded-lg border p-3 text-left {f.enforcement === o.k ? 'border-primary bg-primary-soft' : 'border-border-strong hover:bg-hover'}">
						<div class="text-base font-medium">{o.t}</div>
						<div class="mt-0.5 text-xs text-muted">{o.d}</div>
						{#if f.enforcement === o.k}<Check class="absolute right-3 top-3 size-3.5 text-primary-text" />{/if}
					</button>
				{/each}
			</div>
		</div>
		<div class="flex flex-col gap-1.5">
			<span class="text-sm font-medium">Policy source</span>
			<div class="grid grid-cols-2 gap-3">
				{#each [{ k: 'inline', t: 'Write in Tuffie', d: 'Edit policies here with a Rego editor.' }, { k: 'path', t: 'Folder', d: 'Load *.rego files from a folder, for example a Git checkout.' }] as o (o.k)}
					<button type="button" onclick={() => (f.source_kind = o.k as typeof f.source_kind)} class="relative rounded-lg border p-3 text-left {f.source_kind === o.k ? 'border-primary bg-primary-soft' : 'border-border-strong hover:bg-hover'}">
						<div class="text-base font-medium">{o.t}</div>
						<div class="mt-0.5 text-xs text-muted">{o.d}</div>
						{#if f.source_kind === o.k}<Check class="absolute right-3 top-3 size-3.5 text-primary-text" />{/if}
					</button>
				{/each}
			</div>
			{#if f.source_kind === 'path'}<Input bind:value={f.source_path} placeholder="/home/you/policies/terraform" mono class="mt-1" />{/if}
		</div>
		<div class="flex flex-col gap-1.5"><span class="text-sm font-medium">Scope</span><ScopePicker bind:global={f.global} bind:selected={f.workspace_ids} /></div>
		{#if err}<p class="text-sm text-danger">{err}</p>{/if}
	</form>
	{#snippet footer()}
		<Button onclick={() => (open = false)}>Cancel</Button>
		<Button variant="primary" type="submit" form="psform" loading={saving}>Create policy set</Button>
	{/snippet}
</Dialog>
