<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import { api, errMsg } from '#lib/api';
	import type { VariableSet } from '#lib/types';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import ScopePicker from '#lib/components/ScopePicker.svelte';
	import VariableEditor from '#lib/components/VariableEditor.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Field from '#lib/components/ui/Field.svelte';
	import Input from '#lib/components/ui/Input.svelte';
	import Textarea from '#lib/components/ui/Textarea.svelte';
	import Spinner from '#lib/components/ui/Spinner.svelte';
	import Badge from '#lib/components/ui/Badge.svelte';

	const id = $derived(page.params.id!);
	let set = $state<VariableSet | null>(null);
	let f = $state({ name: '', description: '', global: false, workspace_ids: [] as string[] });
	let saving = $state(false);

	const load = async () => {
		set = await api.variableSet(id);
	};
	$effect(() => {
		api.variableSet(id).then((s) => {
			set = s;
			f = { name: s.name, description: s.description, global: s.global, workspace_ids: [...s.workspace_ids] };
		});
	});

	async function save() {
		saving = true;
		try {
			set = await api.updateVariableSet(id, f);
			toast.success('Variable set saved');
		} catch (e) {
			toast.error(errMsg(e));
		} finally {
			saving = false;
		}
	}
	async function del() {
		if (!confirm(`Delete variable set ${set?.name}? Its variables are deleted too.`)) return;
		try {
			await api.deleteVariableSet(id);
			goto('/variable-sets');
		} catch (e) {
			toast.error(errMsg(e));
		}
	}
</script>

<PageHeader title={set?.name ?? ''} crumbs={[{ label: 'Variable sets', href: '/variable-sets' }, { label: set?.name ?? '…' }]}>
	{#snippet meta()}
		{#if set}<Badge tone={set.global ? 'primary' : 'neutral'}>{set.global ? 'All workspaces' : `${set.workspace_ids.length} workspaces`}</Badge>{/if}
	{/snippet}
	{#snippet actions()}<Button variant="outline-danger" onclick={del}>Delete set</Button>{/snippet}
</PageHeader>

<div class="min-h-0 flex-1 overflow-auto px-8 py-6">
	{#if !set}
		<div class="grid place-items-center py-16"><Spinner /></div>
	{:else}
		<div class="grid gap-5 xl:grid-cols-[minmax(0,1fr)_400px]">
			<div class="min-w-0"><VariableEditor variables={set.variables} create={(b) => api.createSetVariable(id, b)} reload={load} /></div>
			<form class="h-fit overflow-hidden rounded-lg border border-border bg-surface" onsubmit={(e) => (e.preventDefault(), save())}>
				<header class="border-b border-border px-4 py-3"><h3 class="text-[13px] font-semibold">Details and scope</h3></header>
				<div class="flex flex-col gap-4 p-4">
					<Field label="Name" for="n"><Input id="n" bind:value={f.name} /></Field>
					<Field label="Description" for="d"><Textarea id="d" bind:value={f.description} class="min-h-14" /></Field>
					<ScopePicker bind:global={f.global} bind:selected={f.workspace_ids} />
					<p class="text-[12px] text-subtle">Precedence: global sets, then assigned sets (each by name), then workspace variables. The last one wins.</p>
				</div>
				<footer class="flex justify-end border-t border-border bg-surface-2 px-4 py-3"><Button variant="primary" type="submit" loading={saving}>Save</Button></footer>
			</form>
		</div>
	{/if}
</div>
