<script lang="ts">
	import { goto } from '$app/navigation';
	import { api, errMsg } from '#lib/api';
	import { ago } from '#lib/format';
	import type { VariableSet } from '#lib/types';
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
	import Layers from '@lucide/svelte/icons/layers';

	let sets = $state<VariableSet[] | null>(null);
	let open = $state(false);
	let f = $state({ name: '', description: '', global: false, workspace_ids: [] as string[] });
	let saving = $state(false);
	let err = $state('');

	$effect(() => {
		api.variableSets().then((s) => (sets = s));
	});

	async function create() {
		saving = true;
		err = '';
		try {
			const s = await api.createVariableSet(f);
			goto(`/variable-sets/${s.id}`);
		} catch (e) {
			err = errMsg(e);
		} finally {
			saving = false;
		}
	}
</script>

<PageHeader title="Variable sets" subtitle="Share variables, such as cloud credentials or tags, across many workspaces.">
	{#snippet actions()}
		<Button variant="primary" onclick={() => ((f = { name: '', description: '', global: false, workspace_ids: [] }), (err = ''), (open = true))}><Plus />New variable set</Button>
	{/snippet}
</PageHeader>

<div class="min-h-0 flex-1 overflow-auto px-8 py-6">
	{#if sets === null}
		<div class="grid place-items-center py-16"><Spinner /></div>
	{:else if sets.length === 0}
		<div class="rounded-lg border border-dashed border-border-strong bg-surface">
			<Empty icon={Layers} title="No variable sets" text="A variable set is a group of variables that applies to all workspaces, or to the workspaces you choose. Workspace variables override set variables with the same key." />
		</div>
	{:else}
		<div class="overflow-hidden rounded-lg border border-border bg-surface">
			<table class="w-full text-left text-[13px]">
				<thead class="border-b border-border bg-surface-2 text-[11.5px] uppercase tracking-wide text-subtle">
					<tr><th class="px-4 py-2.5 font-medium">Name</th><th class="px-4 py-2.5 font-medium">Applies to</th><th class="px-4 py-2.5 text-right font-medium">Variables</th><th class="px-4 py-2.5 text-right font-medium">Updated</th></tr>
				</thead>
				<tbody class="divide-y divide-border">
					{#each sets as s (s.id)}
						<tr class="relative hover:bg-hover">
							<td class="px-4 py-3">
								<a href="/variable-sets/{s.id}" class="font-medium after:absolute after:inset-0">{s.name}</a>
								{#if s.description}<div class="mt-0.5 text-[12px] text-subtle">{s.description}</div>{/if}
							</td>
							<td class="px-4 py-3">
								{#if s.global}<Badge tone="primary">All workspaces</Badge>{:else}<span class="text-muted">{s.workspace_ids.length} workspace{s.workspace_ids.length === 1 ? '' : 's'}</span>{/if}
							</td>
							<td class="px-4 py-3 text-right tabular text-muted">{s.variables.length}</td>
							<td class="px-4 py-3 text-right text-[12px] text-subtle">{ago(s.updated_at)}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>

<Dialog bind:open title="New variable set" width="max-w-xl">
	<form id="vsform" class="flex flex-col gap-4" onsubmit={(e) => (e.preventDefault(), create())}>
		<Field label="Name" for="n"><Input id="n" bind:value={f.name} placeholder="aws-prod-credentials" required /></Field>
		<Field label="Description" for="d"><Textarea id="d" bind:value={f.description} class="min-h-14" placeholder="Optional" /></Field>
		<div class="flex flex-col gap-1.5"><span class="text-[12.5px] font-medium">Scope</span><ScopePicker bind:global={f.global} bind:selected={f.workspace_ids} /></div>
		{#if err}<p class="text-[12.5px] text-danger">{err}</p>{/if}
	</form>
	{#snippet footer()}
		<Button onclick={() => (open = false)}>Cancel</Button>
		<Button variant="primary" type="submit" form="vsform" loading={saving}>Create variable set</Button>
	{/snippet}
</Dialog>
