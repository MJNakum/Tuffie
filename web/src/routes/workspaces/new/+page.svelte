<script lang="ts">
	import { goto } from '$app/navigation';
	import { api, errMsg } from '#lib/api';
	import { app } from '#lib/app.svelte';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Input from '#lib/components/ui/Input.svelte';
	import Textarea from '#lib/components/ui/Textarea.svelte';
	import Field from '#lib/components/ui/Field.svelte';
	import Switch from '#lib/components/ui/Switch.svelte';
	import Folder from '@lucide/svelte/icons/folder';
	import GitBranch from '@lucide/svelte/icons/git-branch';
	import Check from '@lucide/svelte/icons/check';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';

	let name = $state('');
	let description = $state('');
	let source = $state<'local' | 'git'>('local');
	let localPath = $state('');
	let gitUrl = $state('');
	let gitRef = $state('');
	let workingDir = $state('');
	let tool = $state<'terraform' | 'tofu'>('terraform');
	let toolPath = $state('');
	let autoApply = $state(false);
	let saving = $state(false);
	let error = $state('');

	const tools = $derived(app.system?.tools ?? []);
	$effect(() => {
		// Pick the tool that is installed when only one is.
		const found = tools.filter((t) => t.path);
		if (found.length === 1) tool = found[0].name;
	});

	const nameOk = $derived(/^[A-Za-z0-9_-]{1,90}$/.test(name));

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		error = '';
		saving = true;
		try {
			const ws = await api.createWorkspace({
				name,
				description,
				source_kind: source,
				local_path: source === 'local' ? localPath.trim() : null,
				git_url: source === 'git' ? gitUrl.trim() : null,
				git_ref: source === 'git' ? gitRef.trim() || null : null,
				working_dir: workingDir.trim(),
				tool,
				tool_path: toolPath.trim() || null,
				auto_apply: autoApply
			});
			goto(`/workspaces/${ws.id}`);
		} catch (err) {
			error = errMsg(err);
		} finally {
			saving = false;
		}
	}
</script>

<PageHeader title="New workspace" crumbs={[{ label: 'Workspaces', href: '/' }, { label: 'New' }]} />

<div class="min-h-0 flex-1 overflow-auto px-8 py-6">
	<form onsubmit={submit} class="mx-auto flex max-w-2xl flex-col gap-6">
		<section class="rounded-lg border border-border bg-surface">
			<header class="border-b border-border px-5 py-3"><h2 class="text-[13px] font-semibold">1. Source</h2></header>
			<div class="flex flex-col gap-4 p-5">
				<div class="grid grid-cols-2 gap-3">
					{#each [{ k: 'local', icon: Folder, t: 'Local folder', d: 'A folder on this machine. Tuffie edits files in place.' }, { k: 'git', icon: GitBranch, t: 'Git repository', d: 'Tuffie clones the repo and syncs it on demand.' }] as o (o.k)}
						<button
							type="button"
							onclick={() => (source = o.k as 'local' | 'git')}
							class="relative flex items-start gap-3 rounded-lg border p-3.5 text-left transition-colors {source === o.k
								? 'border-primary bg-primary-soft'
								: 'border-border-strong hover:bg-hover'}"
						>
							<o.icon class="mt-0.5 size-4 {source === o.k ? 'text-primary-text' : 'text-muted'}" />
							<div>
								<div class="text-[13px] font-medium">{o.t}</div>
								<div class="mt-0.5 text-[12px] text-muted">{o.d}</div>
							</div>
							{#if source === o.k}<Check class="absolute right-3 top-3 size-3.5 text-primary-text" />{/if}
						</button>
					{/each}
				</div>
				{#if source === 'local'}
					<Field label="Folder path" for="path" hint="Absolute path to the folder with your .tf files.">
						<Input id="path" bind:value={localPath} placeholder="/home/you/infra/networking" mono required />
					</Field>
				{:else}
					<Field label="Repository URL" for="url" hint="https://, ssh:// or git@. Tuffie uses your normal git credentials (SSH agent or credential helper).">
						<Input id="url" bind:value={gitUrl} placeholder="git@github.com:acme/infra.git" mono required />
					</Field>
					<Field label="Branch, tag or commit" for="ref" hint="Leave empty for the default branch.">
						<Input id="ref" bind:value={gitRef} placeholder="main" mono />
					</Field>
				{/if}
				<Field label="Working directory" for="wd" hint="Optional. Sub-folder that holds the configuration, for example envs/prod.">
					<Input id="wd" bind:value={workingDir} placeholder="." mono />
				</Field>
			</div>
		</section>

		<section class="rounded-lg border border-border bg-surface">
			<header class="border-b border-border px-5 py-3"><h2 class="text-[13px] font-semibold">2. Details</h2></header>
			<div class="flex flex-col gap-4 p-5">
				<Field label="Name" for="name" error={name && !nameOk ? 'Use letters, numbers, - and _ only.' : undefined} hint="Letters, numbers, - and _. For example networking-prod.">
					<Input id="name" bind:value={name} placeholder="networking-prod" required />
				</Field>
				<Field label="Description" for="desc">
					<Textarea id="desc" bind:value={description} placeholder="What does this configuration manage?" class="min-h-16" />
				</Field>
			</div>
		</section>

		<section class="rounded-lg border border-border bg-surface">
			<header class="border-b border-border px-5 py-3"><h2 class="text-[13px] font-semibold">3. Execution</h2></header>
			<div class="flex flex-col gap-4 p-5">
				<div class="flex flex-col gap-1.5">
					<span class="text-[12.5px] font-medium">Tool</span>
					<div class="grid grid-cols-2 gap-3">
						{#each ['terraform', 'tofu'] as t (t)}
							{@const info = tools.find((x) => x.name === t)}
							<button
								type="button"
								onclick={() => (tool = t as 'terraform' | 'tofu')}
								class="flex items-center justify-between rounded-lg border px-3.5 py-2.5 text-left transition-colors {tool === t
									? 'border-primary bg-primary-soft'
									: 'border-border-strong hover:bg-hover'}"
							>
								<div>
									<div class="text-[13px] font-medium">{t === 'tofu' ? 'OpenTofu' : 'Terraform'}</div>
									<div class="mono text-[11.5px] {info?.path ? 'text-muted' : 'text-subtle'}">
										{info?.path ? `v${info.version ?? '?'} · ${info.path}` : 'not found on PATH'}
									</div>
								</div>
								{#if tool === t}<Check class="size-3.5 text-primary-text" />{/if}
							</button>
						{/each}
					</div>
					{#if !tools.find((x) => x.name === tool)?.path}
						<p class="mt-1 flex items-center gap-1.5 text-[12px] text-warning">
							<TriangleAlert class="size-3.5" />{tool} is not on PATH. Set the binary path below.
						</p>
					{/if}
				</div>
				<Field label="Binary path" for="tp" hint="Optional. Use a specific binary instead of the one on PATH.">
					<Input id="tp" bind:value={toolPath} placeholder="/usr/local/bin/{tool}" mono />
				</Field>
				<label class="flex items-start justify-between gap-6 rounded-lg border border-border-strong px-3.5 py-3">
					<div>
						<div class="text-[13px] font-medium">Auto-apply</div>
						<div class="text-[12px] text-muted">Apply a successful plan without asking for confirmation. Keep this off for production.</div>
					</div>
					<Switch bind:checked={autoApply} />
				</label>
			</div>
		</section>

		{#if error}<div class="rounded-md border border-danger/40 bg-danger-soft px-3 py-2 text-[13px] text-danger">{error}</div>{/if}

		<div class="flex justify-end gap-2 pb-10">
			<Button href="/">Cancel</Button>
			<Button variant="primary" type="submit" loading={saving} disabled={!nameOk}>Create workspace</Button>
		</div>
	</form>
</div>
