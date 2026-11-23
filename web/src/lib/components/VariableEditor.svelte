<script lang="ts">
	// Terraform + environment variable tables with add/edit dialog.
	// Used by workspaces and by variable sets.
	import { toast } from 'svelte-sonner';
	import { api, errMsg } from '#lib/api';
	import type { Variable } from '#lib/types';
	import Button from './ui/Button.svelte';
	import Dialog from './ui/Dialog.svelte';
	import Input from './ui/Input.svelte';
	import Textarea from './ui/Textarea.svelte';
	import Field from './ui/Field.svelte';
	import Switch from './ui/Switch.svelte';
	import Badge from './ui/Badge.svelte';
	import Plus from '@lucide/svelte/icons/plus';
	import Pencil from '@lucide/svelte/icons/pencil';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import Lock from '@lucide/svelte/icons/lock';
	import { flip, rise, vanish } from '#lib/motion';

	let {
		variables,
		create,
		reload,
		overridden = new Set<string>()
	}: {
		variables: Variable[];
		create: (body: Partial<Variable>) => Promise<unknown>;
		reload: () => unknown;
		/** `category:key` of variables that a higher-precedence source overrides. */
		overridden?: Set<string>;
	} = $props();

	let open = $state(false);
	let editing = $state<Variable | null>(null);
	let form = $state({ key: '', value: '', category: 'terraform' as 'terraform' | 'env', sensitive: false, hcl: false, description: '' });
	let saving = $state(false);
	let err = $state('');

	function openNew(category: 'terraform' | 'env') {
		editing = null;
		form = { key: '', value: '', category, sensitive: false, hcl: false, description: '' };
		err = '';
		open = true;
	}
	function openEdit(v: Variable) {
		editing = v;
		form = { key: v.key, value: v.value ?? '', category: v.category, sensitive: v.sensitive, hcl: v.hcl, description: v.description };
		err = '';
		open = true;
	}
	async function save() {
		saving = true;
		err = '';
		try {
			if (editing) {
				const body: Partial<Variable> = { key: form.key, sensitive: form.sensitive, hcl: form.hcl, description: form.description };
				// For sensitive variables an empty value means "keep the current value".
				if (!editing.sensitive || form.value !== '') body.value = form.value;
				await api.updateVariable(editing.id, body);
			} else {
				await create(form);
			}
			open = false;
			toast.success(editing ? 'Variable updated' : 'Variable added');
			await reload();
		} catch (e) {
			err = errMsg(e);
		} finally {
			saving = false;
		}
	}
	async function remove(v: Variable) {
		if (!confirm(`Delete variable ${v.key}?`)) return;
		try {
			await api.deleteVariable(v.id);
			await reload();
		} catch (e) {
			toast.error(errMsg(e));
		}
	}
</script>

{#snippet section(category: 'terraform' | 'env', title: string, text: string)}
	{@const list = variables.filter((v) => v.category === category)}
	<section class="overflow-hidden rounded-lg border border-border bg-surface">
		<header class="flex items-center justify-between gap-4 border-b border-border px-4 py-3">
			<div>
				<h3 class="text-base font-semibold">{title}</h3>
				<p class="text-xs text-muted">{text}</p>
			</div>
			<Button size="sm" onclick={() => openNew(category)}><Plus />Add variable</Button>
		</header>
		{#if list.length === 0}
			<p class="px-4 py-6 text-center text-sm text-subtle">No variables.</p>
		{:else}
			<table class="w-full table-fixed text-sm">
				<thead class="border-b border-border text-2xs uppercase tracking-wide text-subtle">
					<tr><th class="w-[34%] px-4 py-2 text-left font-medium">Key</th><th class="px-4 py-2 text-left font-medium">Value</th><th class="w-[26%] px-4 py-2 text-left font-medium">Description</th><th class="w-20"></th></tr>
				</thead>
				<tbody class="divide-y divide-border">
					{#each list as v (v.id)}
						{@const over = overridden.has(`${v.category}:${v.key}`)}
						<tr class="group hover:bg-hover/50" animate:flip in:rise out:vanish>
							<td class="px-4 py-2.5">
								<span class="mono font-medium {over ? 'text-subtle line-through' : ''}">{v.key}</span>
								<span class="ml-2 inline-flex gap-1">
									{#if v.sensitive}<Badge><Lock class="size-3" />sensitive</Badge>{/if}
									{#if v.hcl}<Badge tone="primary">HCL</Badge>{/if}
									{#if over}<Badge tone="warning">overridden</Badge>{/if}
								</span>
							</td>
							<td class="mono truncate px-4 py-2.5 {v.sensitive ? 'text-subtle' : ''}">{v.sensitive ? '••••••••' : v.value}</td>
							<td class="truncate px-4 py-2.5 text-muted">{v.description}</td>
							<td class="px-2 py-1.5 text-right opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
								<Button size="icon-sm" variant="ghost" onclick={() => openEdit(v)} title="Edit"><Pencil /></Button>
								<Button size="icon-sm" variant="ghost" onclick={() => remove(v)} title="Delete"><Trash2 /></Button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
	</section>
{/snippet}

<div class="flex flex-col gap-5">
	{@render section('terraform', 'Terraform variables', 'Set as TF_VAR_<key>. They fill the variable blocks in your configuration.')}
	{@render section('env', 'Environment variables', 'Set in the process environment, for example AWS_PROFILE or ARM_CLIENT_ID.')}
</div>

<Dialog bind:open title={editing ? `Edit ${editing.key}` : form.category === 'env' ? 'Add environment variable' : 'Add Terraform variable'}>
	<form id="varform" class="flex flex-col gap-4" onsubmit={(e) => (e.preventDefault(), save())}>
		<Field label="Key" for="k"><Input id="k" bind:value={form.key} mono required placeholder={form.category === 'env' ? 'AWS_PROFILE' : 'instance_type'} /></Field>
		<Field label="Value" for="v" hint={editing?.sensitive ? 'Leave empty to keep the current value.' : form.hcl ? 'Parsed as HCL, for example ["a", "b"] or { size = 2 }.' : undefined}>
			<Textarea id="v" bind:value={form.value} mono class="min-h-16" placeholder={editing?.sensitive ? '••••••••' : ''} />
		</Field>
		<Field label="Description" for="d"><Input id="d" bind:value={form.description} placeholder="Optional" /></Field>
		<div class="flex flex-col gap-2.5 rounded-lg border border-border-strong p-3">
			{#if form.category === 'terraform'}
				<label class="flex items-center justify-between gap-4 text-sm"><span><b class="font-medium">HCL</b> <span class="text-muted">— parse the value as HCL instead of a string</span></span><Switch bind:checked={form.hcl} /></label>
			{/if}
			<label class="flex items-center justify-between gap-4 text-sm">
				<span><b class="font-medium">Sensitive</b> <span class="text-muted">— write-only, hidden in logs</span></span>
				<Switch bind:checked={form.sensitive} disabled={editing?.sensitive} />
			</label>
		</div>
		{#if err}<p class="text-sm text-danger">{err}</p>{/if}
	</form>
	{#snippet footer()}
		<Button onclick={() => (open = false)}>Cancel</Button>
		<Button variant="primary" type="submit" form="varform" loading={saving}>{editing ? 'Save' : 'Add variable'}</Button>
	{/snippet}
</Dialog>
