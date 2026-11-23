<script lang="ts">
	import { page } from '$app/state';
	import { goto, beforeNavigate } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import { api, errMsg } from '#lib/api';
	import { ago, kindLabel } from '#lib/format';
	import type { PolicySet, PolicySetResult } from '#lib/types';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import ScopePicker from '#lib/components/ScopePicker.svelte';
	import PolicyResults from '#lib/components/PolicyResults.svelte';
	import CodeEditor from '#lib/components/CodeEditor.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Field from '#lib/components/ui/Field.svelte';
	import Input from '#lib/components/ui/Input.svelte';
	import Textarea from '#lib/components/ui/Textarea.svelte';
	import Select from '#lib/components/ui/Select.svelte';
	import Spinner from '#lib/components/ui/Spinner.svelte';
	import Badge from '#lib/components/ui/Badge.svelte';
	import FileCode from '@lucide/svelte/icons/file-code-2';
	import FilePlus from '@lucide/svelte/icons/file-plus';
	import Save from '@lucide/svelte/icons/save';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import FlaskConical from '@lucide/svelte/icons/flask-conical';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';

	const TEMPLATE = `package terraform.example

import rego.v1

# Fail when a new S3 bucket has no "owner" tag.
# input is the plan JSON from \`terraform show -json\`.
deny contains msg if {
	some rc in input.resource_changes
	rc.type == "aws_s3_bucket"
	"create" in rc.change.actions
	not rc.change.after.tags.owner
	msg := sprintf("%s must have an owner tag", [rc.address])
}

# Warnings never block an apply.
warn contains msg if {
	some rc in input.resource_changes
	"delete" in rc.change.actions
	msg := sprintf("%s will be destroyed", [rc.address])
}
`;

	const id = $derived(page.params.id!);
	let set = $state<PolicySet | null>(null);
	let f = $state({ name: '', description: '', enforcement: 'advisory', source_path: '', global: false, workspace_ids: [] as string[] });
	let savingSet = $state(false);

	let current = $state<string | null>(null);
	let original = $state('');
	let content = $state('');
	let compileErr = $state('');
	let savingPolicy = $state(false);
	const dirty = $derived(current !== null && content !== original);

	let runs = $state<{ run_id: string; workspace: string; kind: string; message: string; created_at: string }[]>([]);
	let testRun = $state('');
	let testing = $state(false);
	let result = $state<PolicySetResult | null>(null);

	async function load(selectFirst = false) {
		const s = await api.policySet(id);
		set = s;
		if (selectFirst || (current && !s.policies.find((p) => p.id === current))) {
			const p = s.policies[0];
			select(p?.id ?? null, true);
		}
		return s;
	}

	$effect(() => {
		load(true).then((s) => {
			f = {
				name: s.name,
				description: s.description,
				enforcement: s.enforcement,
				source_path: s.source_path ?? '',
				global: s.global,
				workspace_ids: [...s.workspace_ids]
			};
		});
		api.testableRuns().then((r) => {
			runs = r;
			testRun = r[0]?.run_id ?? '';
		});
	});

	function select(pid: string | null, force = false) {
		if (!force && dirty && !confirm('Discard unsaved changes?')) return;
		current = pid;
		const p = set?.policies.find((x) => x.id === pid);
		original = p?.rego ?? '';
		content = original;
		compileErr = '';
	}

	async function newPolicy() {
		const name = prompt('Policy file name, for example s3-tags.rego');
		if (!name) return;
		try {
			const p = await api.createPolicy(id, { name, rego: TEMPLATE });
			await load();
			select(p.id, true);
		} catch (e) {
			toast.error(errMsg(e));
		}
	}

	async function savePolicy() {
		if (!current || !dirty) return;
		savingPolicy = true;
		compileErr = '';
		try {
			await api.updatePolicy(current, { rego: content });
			original = content;
			await load();
			toast.success('Policy saved');
		} catch (e) {
			compileErr = errMsg(e);
		} finally {
			savingPolicy = false;
		}
	}

	async function deletePolicy() {
		const p = set?.policies.find((x) => x.id === current);
		if (!p || !confirm(`Delete policy ${p.name}?`)) return;
		await api.deletePolicy(p.id);
		await load();
		select(set?.policies[0]?.id ?? null, true);
	}

	async function saveSet(e: SubmitEvent) {
		e.preventDefault();
		savingSet = true;
		try {
			set = await api.updatePolicySet(id, {
				name: f.name,
				description: f.description,
				enforcement: f.enforcement as PolicySet['enforcement'],
				global: f.global,
				workspace_ids: f.workspace_ids,
				...(set?.source_kind === 'path' ? { source_path: f.source_path } : {})
			});
			toast.success('Policy set saved');
		} catch (err) {
			toast.error(errMsg(err));
		} finally {
			savingSet = false;
		}
	}

	async function delSet() {
		if (!confirm(`Delete policy set ${set?.name}?`)) return;
		await api.deletePolicySet(id);
		goto('/policies');
	}

	async function runTest() {
		if (!testRun) return;
		if (dirty) await savePolicy();
		testing = true;
		try {
			result = await api.testPolicySet(id, testRun);
		} catch (e) {
			toast.error(errMsg(e));
		} finally {
			testing = false;
		}
	}

	beforeNavigate((nav) => {
		if (dirty && !confirm('You have unsaved changes. Leave anyway?')) nav.cancel();
	});
</script>

<PageHeader title={set?.name ?? ''} crumbs={[{ label: 'Policies', href: '/policies' }, { label: set?.name ?? '…' }]}>
	{#snippet meta()}
		{#if set}
			<Badge tone={set.enforcement === 'mandatory' ? 'danger' : 'neutral'}>{set.enforcement}</Badge>
			<span>{set.global ? 'All workspaces' : `${set.workspace_ids.length} workspaces`}</span>
			<span>{set.source_kind === 'path' ? 'Folder source' : 'Written in Tuffie'}</span>
		{/if}
	{/snippet}
	{#snippet actions()}<Button variant="outline-danger" onclick={delSet}>Delete set</Button>{/snippet}
</PageHeader>

<div class="min-h-0 flex-1 overflow-auto px-8 py-6">
	{#if !set}
		<div class="grid place-items-center py-16"><Spinner /></div>
	{:else}
		<div class="grid gap-5 xl:grid-cols-[minmax(0,1fr)_380px]">
			<div class="flex min-w-0 flex-col gap-5">
				{#if set.source_kind === 'inline'}
					<div class="grid h-[540px] grid-cols-[210px_minmax(0,1fr)] overflow-hidden rounded-lg border border-border bg-surface">
						<div class="flex min-h-0 flex-col border-r border-border">
							<div class="flex h-10 items-center justify-between border-b border-border pl-3 pr-1.5">
								<span class="text-[11.5px] font-medium uppercase tracking-wide text-subtle">Policies</span>
								<Button size="icon-sm" variant="ghost" title="New policy" onclick={newPolicy}><FilePlus /></Button>
							</div>
							<div class="min-h-0 flex-1 overflow-auto py-1">
								{#each set.policies as p (p.id)}
									<button
										class="flex w-full items-center gap-1.5 px-3 py-1.5 text-left text-[12.5px] {current === p.id ? 'bg-primary-soft text-primary-text' : 'hover:bg-hover'}"
										onclick={() => select(p.id)}
									>
										<FileCode class="size-3.5 shrink-0 {current === p.id ? '' : 'text-subtle'}" /><span class="truncate">{p.name}</span>
									</button>
								{:else}
									<div class="px-3 py-4 text-[12.5px] text-subtle">
										No policies yet.
										<button class="mt-2 block text-primary-text hover:underline" onclick={newPolicy}>Add one from a template</button>
									</div>
								{/each}
							</div>
						</div>
						<div class="flex min-h-0 flex-col">
							<div class="flex h-10 shrink-0 items-center justify-between gap-3 border-b border-border px-3">
								<span class="mono flex items-center gap-2 truncate text-[12.5px]">
									{set.policies.find((p) => p.id === current)?.name ?? 'No policy selected'}
									{#if dirty}<span class="size-1.5 rounded-full bg-warning"></span>{/if}
								</span>
								{#if current}
									<div class="flex items-center gap-1.5">
										<Button size="icon-sm" variant="ghost" title="Delete policy" onclick={deletePolicy}><Trash2 /></Button>
										<Button size="sm" variant={dirty ? 'primary' : 'secondary'} disabled={!dirty} loading={savingPolicy} onclick={savePolicy}><Save />Save</Button>
									</div>
								{/if}
							</div>
							{#if compileErr}
								<pre class="max-h-32 shrink-0 overflow-auto whitespace-pre-wrap border-b border-danger/30 bg-danger-soft px-3 py-2 font-mono text-[11.5px] text-danger">{compileErr}</pre>
							{/if}
							<div class="min-h-0 flex-1">
								{#if current}
									<CodeEditor value={original} path={set.policies.find((p) => p.id === current)?.name ?? 'policy.rego'} onChange={(v) => (content = v)} onSave={savePolicy} />
								{:else}
									<div class="grid h-full place-items-center text-[13px] text-subtle">Select or add a policy.</div>
								{/if}
							</div>
						</div>
					</div>
				{:else}
					<section class="overflow-hidden rounded-lg border border-border bg-surface">
						<header class="border-b border-border px-4 py-3">
							<h3 class="text-[13px] font-semibold">Policy files</h3>
							<p class="mono text-[12px] text-muted">{set.source_path}</p>
						</header>
						{#if set.load_error}
							<div class="flex items-start gap-2 px-4 py-3 text-[12.5px] text-danger"><TriangleAlert class="mt-0.5 size-4" />{set.load_error}</div>
						{:else}
							<ul class="divide-y divide-border">
								{#each set.files ?? [] as file (file)}
									<li class="mono flex items-center gap-2 px-4 py-2 text-[12.5px]"><FileCode class="size-3.5 text-subtle" />{file}</li>
								{:else}
									<li class="px-4 py-4 text-[12.5px] text-subtle">No *.rego files found. Files that end in _test.rego are skipped.</li>
								{/each}
							</ul>
						{/if}
					</section>
				{/if}

				<section class="overflow-hidden rounded-lg border border-border bg-surface">
					<header class="flex flex-wrap items-center justify-between gap-3 border-b border-border px-4 py-3">
						<div>
							<h3 class="flex items-center gap-2 text-[13px] font-semibold"><FlaskConical class="size-4 text-muted" />Test against a plan</h3>
							<p class="text-[12px] text-muted">Run this policy set on the saved plan of an earlier run. Nothing is applied.</p>
						</div>
						<div class="flex items-center gap-2">
							<Select
								bind:value={testRun}
								class="w-80"
								options={runs.length
									? runs.map((r) => ({ value: r.run_id, label: `${r.workspace} · ${r.message || kindLabel[r.kind as keyof typeof kindLabel]} · ${ago(r.created_at)}` }))
									: [{ value: '', label: 'No runs with a saved plan' }]}
							/>
							<Button variant="primary" loading={testing} disabled={!testRun} onclick={runTest}>Run test</Button>
						</div>
					</header>
					{#if result}
						<PolicyResults sets={[result]} />
					{:else}
						<p class="px-4 py-5 text-[12.5px] text-subtle">Choose a run and click Run test.</p>
					{/if}
				</section>
			</div>

			<form class="h-fit overflow-hidden rounded-lg border border-border bg-surface" onsubmit={saveSet}>
				<header class="border-b border-border px-4 py-3"><h3 class="text-[13px] font-semibold">Settings</h3></header>
				<div class="flex flex-col gap-4 p-4">
					<Field label="Name" for="n"><Input id="n" bind:value={f.name} /></Field>
					<Field label="Description" for="d"><Textarea id="d" bind:value={f.description} class="min-h-14" /></Field>
					<Field label="Enforcement" for="e" hint="Mandatory: a failing deny rule blocks the apply. Advisory: failures are warnings.">
						<Select id="e" bind:value={f.enforcement} options={[{ value: 'advisory', label: 'Advisory' }, { value: 'mandatory', label: 'Mandatory' }]} />
					</Field>
					{#if set.source_kind === 'path'}
						<Field label="Policy folder" for="p"><Input id="p" bind:value={f.source_path} mono /></Field>
					{/if}
					<ScopePicker bind:global={f.global} bind:selected={f.workspace_ids} />
				</div>
				<footer class="flex justify-end border-t border-border bg-surface-2 px-4 py-3"><Button variant="primary" type="submit" loading={savingSet}>Save</Button></footer>
			</form>
		</div>
	{/if}
</div>
