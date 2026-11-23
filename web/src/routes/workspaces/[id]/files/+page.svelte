<script lang="ts">
	import { conceal, reveal } from '#lib/motion';
	import Skeleton from '#lib/components/ui/Skeleton.svelte';
	import { toast } from 'svelte-sonner';
	import { beforeNavigate } from '$app/navigation';
	import { api, errMsg } from '#lib/api';
	import { useWorkspace } from '#lib/workspace.svelte';
	import type { FileEntry } from '#lib/types';
	import CodeEditor from '#lib/components/CodeEditor.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Spinner from '#lib/components/ui/Spinner.svelte';
	import Kbd from '#lib/components/ui/Kbd.svelte';
	import FileIcon from '@lucide/svelte/icons/file';
	import FileCode from '@lucide/svelte/icons/file-code-2';
	import FolderIcon from '@lucide/svelte/icons/folder';
	import FolderOpen from '@lucide/svelte/icons/folder-open';
	import FilePlus from '@lucide/svelte/icons/file-plus';
	import Save from '@lucide/svelte/icons/save';

	const ctx = useWorkspace();
	let tree = $state<FileEntry[] | null>(null);
	let openDirs = $state<Record<string, boolean>>({});
	let path = $state<string | null>(null);
	let original = $state('');
	let content = $state('');
	let loadingFile = $state(false);
	let saving = $state(false);
	const dirty = $derived(path !== null && content !== original);

	const loadTree = async () => {
		try {
			tree = await api.files(ctx.id);
			if (!path) {
				const first = findFirst(tree, (f) => f.name === 'main.tf') ?? findFirst(tree, (f) => f.name.endsWith('.tf'));
				if (first) open(first.path);
			}
		} catch (e) {
			tree = [];
			toast.error(errMsg(e));
		}
	};
	$effect(() => {
		loadTree();
	});

	function findFirst(list: FileEntry[], pred: (f: FileEntry) => boolean): FileEntry | null {
		for (const f of list) {
			if (!f.dir && pred(f)) return f;
			if (f.children) {
				const r = findFirst(f.children, pred);
				if (r) return r;
			}
		}
		return null;
	}

	async function open(p: string) {
		if (dirty && !confirm('Discard unsaved changes?')) return;
		loadingFile = true;
		try {
			const f = await api.readFile(ctx.id, p);
			original = f.content;
			content = f.content;
			path = p;
		} catch (e) {
			toast.error(errMsg(e));
		} finally {
			loadingFile = false;
		}
	}

	async function save() {
		if (!path || !dirty) return;
		saving = true;
		try {
			await api.writeFile(ctx.id, path, content);
			original = content;
			toast.success(`Saved ${path}`);
		} catch (e) {
			toast.error(errMsg(e));
		} finally {
			saving = false;
		}
	}

	async function newFile() {
		const p = prompt('New file path (relative to the workspace), for example variables.tf');
		if (!p) return;
		try {
			await api.writeFile(ctx.id, p, '');
			await loadTree();
			open(p);
		} catch (e) {
			toast.error(errMsg(e));
		}
	}

	beforeNavigate((nav) => {
		if (dirty && !confirm('You have unsaved changes. Leave anyway?')) nav.cancel();
	});

	const isCode = (n: string) => /\.(tf|tfvars|hcl|json|ya?ml|sh)$/.test(n);
</script>

{#snippet node(f: FileEntry, depth: number)}
	{#if f.dir}
		<button
			class="flex w-full items-center gap-1.5 py-1 pr-2 text-left text-sm text-muted hover:bg-hover hover:text-fg"
			style="padding-left: {8 + depth * 12}px"
			onclick={() => (openDirs[f.path] = !openDirs[f.path])}
		>
			{#if openDirs[f.path]}<FolderOpen class="size-3.5 shrink-0" />{:else}<FolderIcon class="size-3.5 shrink-0" />{/if}
			<span class="truncate">{f.name}</span>
		</button>
		{#if openDirs[f.path]}
			<div in:reveal={{ duration: 200 }} out:conceal={{ duration: 200 }}>
				{#each f.children ?? [] as c (c.path)}{@render node(c, depth + 1)}{/each}
			</div>
		{/if}
	{:else}
		<button
			class="flex w-full items-center gap-1.5 py-1 pr-2 text-left text-sm {path === f.path ? 'bg-primary-soft text-primary-text' : 'text-fg hover:bg-hover'}"
			style="padding-left: {8 + depth * 12}px"
			onclick={() => open(f.path)}
		>
			{#if isCode(f.name)}<FileCode class="size-3.5 shrink-0 {path === f.path ? '' : 'text-subtle'}" />{:else}<FileIcon class="size-3.5 shrink-0 text-subtle" />{/if}
			<span class="truncate">{f.name}</span>
		</button>
	{/if}
{/snippet}

<div class="flex h-full flex-col px-8 py-6">
	<div class="grid min-h-[520px] flex-1 grid-cols-[240px_minmax(0,1fr)] overflow-hidden rounded-lg border border-border bg-surface">
		<div class="flex min-h-0 flex-col border-r border-border">
			<div class="flex h-10 items-center justify-between border-b border-border pl-3 pr-1.5">
				<span class="text-2xs font-medium uppercase tracking-wide text-subtle">Files</span>
				<Button size="icon-sm" variant="ghost" title="New file" onclick={newFile}><FilePlus /></Button>
			</div>
			<div class="min-h-0 flex-1 overflow-auto py-1">
				{#if tree === null}
					<div class="flex flex-col gap-2.5 px-3 py-2">{#each [70, 55, 80, 45, 60] as w (w)}<Skeleton class="h-3" style="width: {w}%" />{/each}</div>
				{:else}
					{#each tree as f (f.path)}{@render node(f, 0)}{/each}
				{/if}
			</div>
		</div>
		<div class="flex min-h-0 flex-col">
			<div class="flex h-10 shrink-0 items-center justify-between gap-3 border-b border-border px-3">
				<span class="mono flex min-w-0 items-center gap-2 truncate text-sm">
					{path ?? 'No file open'}
					{#if dirty}<span class="size-1.5 shrink-0 rounded-full bg-warning" title="Unsaved changes"></span>{/if}
				</span>
				<div class="flex items-center gap-2">
					{#if dirty}<span class="hidden text-2xs text-subtle sm:inline"><Kbd>Ctrl</Kbd> <Kbd>S</Kbd> to save</span>{/if}
					<Button size="sm" variant={dirty ? 'primary' : 'secondary'} disabled={!dirty} loading={saving} onclick={save}><Save />Save</Button>
				</div>
			</div>
			<div class="relative min-h-0 flex-1">
				{#if loadingFile}<div class="motion-fade absolute inset-0 z-10 grid place-items-center bg-surface/60"><Spinner /></div>{/if}
				{#if path !== null}
					<CodeEditor value={original} {path} onChange={(v) => (content = v)} onSave={save} />
				{:else}
					<div class="grid h-full place-items-center text-base text-subtle">Select a file to edit.</div>
				{/if}
			</div>
		</div>
	</div>
</div>
