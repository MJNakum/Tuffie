<script lang="ts">
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import { api, errMsg } from '#lib/api';
	import { useWorkspace } from '#lib/workspace.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Input from '#lib/components/ui/Input.svelte';
	import Textarea from '#lib/components/ui/Textarea.svelte';
	import Field from '#lib/components/ui/Field.svelte';
	import Select from '#lib/components/ui/Select.svelte';
	import Switch from '#lib/components/ui/Switch.svelte';
	import Dialog from '#lib/components/ui/Dialog.svelte';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import { driftIntervals } from '#lib/format';

	const ctx = useWorkspace();
	// svelte-ignore state_referenced_locally
	const w = ctx.ws!;
	let f = $state({
		name: w.name,
		description: w.description,
		working_dir: w.working_dir,
		git_ref: w.git_ref ?? '',
		tool: w.tool as string,
		tool_path: w.tool_path ?? '',
		auto_apply: w.auto_apply,
		drift_interval: String(w.drift_interval_minutes)
	});
	let saving = $state(false);
	let syncing = $state(false);
	let delOpen = $state(false);
	let delName = $state('');
	let deleting = $state(false);

	async function save(e: SubmitEvent) {
		e.preventDefault();
		saving = true;
		try {
			await api.updateWorkspace(ctx.id, {
				name: f.name,
				description: f.description,
				working_dir: f.working_dir,
				tool: f.tool as 'terraform' | 'tofu',
				tool_path: f.tool_path || null,
				auto_apply: f.auto_apply,
				drift_interval_minutes: Number(f.drift_interval),
				...(ctx.ws?.source_kind === 'git' ? { git_ref: f.git_ref } : {})
			});
			await ctx.reload();
			toast.success('Settings saved');
		} catch (err) {
			toast.error(errMsg(err));
		} finally {
			saving = false;
		}
	}

	async function sync() {
		syncing = true;
		try {
			const r = await api.syncWorkspace(ctx.id);
			toast.success(`Checked out ${r.sha.slice(0, 8)}`);
			ctx.reload();
		} catch (err) {
			toast.error(errMsg(err));
		} finally {
			syncing = false;
		}
	}

	async function del() {
		deleting = true;
		try {
			await api.deleteWorkspace(ctx.id);
			toast.success('Workspace deleted');
			goto('/');
		} catch (err) {
			toast.error(errMsg(err));
			deleting = false;
		}
	}
</script>

<div class="mx-auto flex max-w-3xl flex-col gap-6 px-8 py-6">
	<form onsubmit={save} class="overflow-hidden rounded-lg border border-border bg-surface">
		<header class="border-b border-border px-5 py-3"><h2 class="text-base font-semibold">General</h2></header>
		<div class="flex flex-col gap-4 p-5">
			<Field label="Name" for="n"><Input id="n" bind:value={f.name} /></Field>
			<Field label="Description" for="d"><Textarea id="d" bind:value={f.description} class="min-h-16" /></Field>
			<Field label="Working directory" for="wd" hint="Sub-folder of the source that holds the configuration."><Input id="wd" bind:value={f.working_dir} mono placeholder="." /></Field>
			{#if ctx.ws?.source_kind === 'git'}
				<Field label="Branch, tag or commit" for="ref" hint="Saved now; checked out on the next sync."><Input id="ref" bind:value={f.git_ref} mono placeholder="default branch" /></Field>
			{/if}
			<div class="grid grid-cols-2 gap-4">
				<Field label="Tool" for="t">
					<Select id="t" bind:value={f.tool} options={[{ value: 'terraform', label: 'Terraform' }, { value: 'tofu', label: 'OpenTofu' }]} />
				</Field>
				<Field label="Binary path" for="tp" hint="Empty = find on PATH."><Input id="tp" bind:value={f.tool_path} mono /></Field>
			</div>
			<label class="flex items-start justify-between gap-6 rounded-lg border border-border-strong px-3.5 py-3">
				<div>
					<div class="text-base font-medium">Auto-apply</div>
					<div class="text-xs text-muted">Apply successful plans without confirmation.</div>
				</div>
				<Switch bind:checked={f.auto_apply} />
			</label>
			<Field label="Drift detection" for="drift" hint="Runs a refresh-only plan on a schedule. It never changes your infrastructure or state. A check is skipped while another run is active.">
				<Select id="drift" bind:value={f.drift_interval} options={driftIntervals} />
			</Field>
		</div>
		<footer class="flex justify-end border-t border-border bg-surface-2 px-5 py-3">
			<Button variant="primary" type="submit" loading={saving}>Save changes</Button>
		</footer>
	</form>

	{#if ctx.ws?.source_kind === 'git'}
		<section class="flex items-center justify-between gap-6 rounded-lg border border-border bg-surface px-5 py-4">
			<div>
				<h2 class="text-base font-semibold">Sync repository</h2>
				<p class="text-sm text-muted">Fetch from <span class="mono">{ctx.ws.git_url}</span> and check out <span class="mono">{ctx.ws.git_ref || 'the default branch'}</span>. Local edits are overwritten.</p>
			</div>
			<Button loading={syncing} onclick={sync}><RefreshCw />Sync now</Button>
		</section>
	{/if}

	<section class="overflow-hidden rounded-lg border border-danger/40 bg-surface">
		<header class="border-b border-danger/30 bg-danger-soft px-5 py-3"><h2 class="text-base font-semibold text-danger">Danger zone</h2></header>
		<div class="flex items-center justify-between gap-6 px-5 py-4">
			<div>
				<div class="text-base font-medium">Delete this workspace</div>
				<p class="text-sm text-muted">
					Removes runs, logs and variables from Tuffie. Your infrastructure and state are not touched.
					{#if ctx.ws?.source_kind === 'local'}Your local folder is not deleted.{:else}The Tuffie checkout of the repository is deleted.{/if}
				</p>
			</div>
			<Button variant="outline-danger" onclick={() => ((delName = ''), (delOpen = true))}>Delete workspace</Button>
		</div>
	</section>
</div>

<Dialog bind:open={delOpen} title="Delete workspace" description="This cannot be undone.">
	<Field label="Type the workspace name to confirm" for="dn"><Input id="dn" bind:value={delName} placeholder={ctx.ws?.name} mono /></Field>
	{#snippet footer()}
		<Button onclick={() => (delOpen = false)}>Cancel</Button>
		<Button variant="danger" disabled={delName !== ctx.ws?.name} loading={deleting} onclick={del}>Delete workspace</Button>
	{/snippet}
</Dialog>
