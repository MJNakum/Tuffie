<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { onDestroy } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { api, errMsg } from '#lib/api';
	import { poll } from '#lib/app.svelte';
	import { WorkspaceCtx, setWorkspace } from '#lib/workspace.svelte';
	import type { RunKind } from '#lib/types';
	import Terminal from '#lib/components/Terminal.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Menu from '#lib/components/ui/Menu.svelte';
	import Kbd from '#lib/components/ui/Kbd.svelte';
	import Spinner from '#lib/components/ui/Spinner.svelte';
	import Play from '@lucide/svelte/icons/play';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import FileSearch from '@lucide/svelte/icons/file-search';
	import Rocket from '@lucide/svelte/icons/rocket';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import Radar from '@lucide/svelte/icons/radar';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import SquareTerminal from '@lucide/svelte/icons/square-terminal';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import Folder from '@lucide/svelte/icons/folder';
	import GitBranch from '@lucide/svelte/icons/git-branch';
	import Lock from '@lucide/svelte/icons/lock';
	import X from '@lucide/svelte/icons/x';
	import Minus from '@lucide/svelte/icons/minus';

	let { children } = $props();
	// svelte-ignore state_referenced_locally
	const ctx = new WorkspaceCtx(page.params.id!);
	setWorkspace(ctx);

	$effect(() => {
		const id = page.params.id!;
		if (id !== ctx.id) {
			ctx.id = id;
			ctx.ws = null;
		}
		return poll(() => ctx.reload(), 5000);
	});

	const base = $derived(`/workspaces/${page.params.id}`);
	const tabs = $derived([
		{ href: base, label: 'Overview', exact: true },
		{ href: `${base}/runs`, label: 'Runs' },
		{ href: `${base}/state`, label: 'State' },
		{ href: `${base}/variables`, label: 'Variables' },
		{ href: `${base}/files`, label: 'Files' },
		{ href: `${base}/settings`, label: 'Settings' }
	]);
	const isActive = (t: { href: string; exact?: boolean }) =>
		t.exact ? page.url.pathname === t.href : page.url.pathname.startsWith(t.href);

	let starting = $state(false);
	async function start(kind: RunKind) {
		starting = true;
		try {
			const run = await api.createRun(ctx.id, kind);
			goto(`${base}/runs/${run.id}`);
		} catch (e) {
			toast.error(errMsg(e));
		} finally {
			starting = false;
		}
	}
	const runMenu = [
		{ label: 'Plan and apply', hint: 'Plan, then apply after you confirm', icon: Rocket, onSelect: () => start('plan_and_apply') },
		{ label: 'Plan only', hint: 'Show changes. Never applies.', icon: FileSearch, onSelect: () => start('plan_only') },
		{ label: 'Check for drift', hint: 'Compare real infrastructure with state. Changes nothing.', icon: Radar, onSelect: () => start('drift') },
		{ label: 'Refresh state', hint: 'Accept drift: update the state to match', icon: RefreshCw, onSelect: () => start('refresh') },
		'separator' as const,
		{ label: 'Destroy infrastructure', hint: 'Plan to delete every resource', icon: Trash2, danger: true, onSelect: () => start('destroy') }
	];

	// Terminal drawer
	let termOpen = $state(false);
	let termMounted = $state(false);
	let termHeight = $state(320);
	let term: ReturnType<typeof Terminal> | undefined = $state();

	function toggleTerm() {
		termOpen = !termOpen;
		if (termOpen) {
			termMounted = true;
			setTimeout(() => term?.focus(), 50);
		}
	}
	function onKey(e: KeyboardEvent) {
		if (e.ctrlKey && e.key === '`') {
			e.preventDefault();
			toggleTerm();
		}
	}
	function startDrag(e: PointerEvent) {
		const startY = e.clientY;
		const startH = termHeight;
		const move = (ev: PointerEvent) => (termHeight = Math.min(window.innerHeight - 160, Math.max(140, startH + startY - ev.clientY)));
		const up = () => {
			window.removeEventListener('pointermove', move);
			window.removeEventListener('pointerup', up);
		};
		window.addEventListener('pointermove', move);
		window.addEventListener('pointerup', up);
	}
	onDestroy(() => {});
</script>

<svelte:window onkeydown={onKey} />

<header class="shrink-0 border-b border-border bg-surface px-8 pt-5">
	<nav class="mb-2 flex items-center gap-1 text-[12px] text-subtle">
		<a href="/" class="hover:text-fg">Workspaces</a>
		<ChevronRight class="size-3" />
		<span>{ctx.ws?.name ?? '…'}</span>
	</nav>
	<div class="flex items-start justify-between gap-6">
		<div class="min-w-0">
			<div class="flex items-center gap-2.5">
				<h1 class="truncate text-[20px] font-semibold tracking-[-0.02em]">{ctx.ws?.name ?? ''}</h1>
				{#if ctx.ws?.lock_run_id}
					<a
						href="{base}/runs/{ctx.ws.lock_run_id}"
						class="inline-flex h-[22px] items-center gap-1 rounded-full bg-warning-soft px-2 text-[11.5px] font-medium text-warning"
						title="A run holds the workspace lock"><Lock class="size-3" />Locked</a
					>
				{/if}
			</div>
			{#if ctx.ws}
				<div class="mt-1.5 flex flex-wrap items-center gap-x-4 gap-y-1 text-[12px] text-muted">
					<span class="inline-flex items-center gap-1.5">
						{#if ctx.ws.source_kind === 'git'}<GitBranch class="size-3.5" />{:else}<Folder class="size-3.5" />{/if}
						<span class="mono">{ctx.ws.resolved_dir}</span>
					</span>
					<span class="mono">{ctx.ws.tool} {ctx.ws.tool_version ? `v${ctx.ws.tool_version}` : '(not found)'}</span>
					{#if ctx.ws.auto_apply}<span class="text-warning">Auto-apply on</span>{/if}
				</div>
			{/if}
		</div>
		<div class="flex shrink-0 items-center gap-2">
			<Button onclick={toggleTerm} variant={termOpen ? 'secondary' : 'ghost'}>
				<SquareTerminal />Terminal <Kbd>Ctrl `</Kbd>
			</Button>
			<Menu items={runMenu}>
				{#snippet trigger(props)}
					<Button variant="primary" loading={starting} {...props}><Play />Start run<ChevronDown class="-mr-1 opacity-70" /></Button>
				{/snippet}
			</Menu>
		</div>
	</div>
	<nav class="-mb-px mt-4 flex gap-5">
		{#each tabs as t (t.href)}
			<a
				href={t.href}
				class="border-b-2 pb-2.5 text-[13px] font-medium transition-colors {isActive(t)
					? 'border-primary text-fg'
					: 'border-transparent text-muted hover:text-fg'}">{t.label}</a
			>
		{/each}
	</nav>
</header>

<div class="relative flex min-h-0 flex-1 flex-col">
	<div class="min-h-0 flex-1 overflow-auto">
		{#if ctx.error && !ctx.ws}
			<div class="m-8 rounded-md border border-danger/40 bg-danger-soft px-3 py-2 text-[13px] text-danger">{ctx.error}</div>
		{:else if !ctx.ws}
			<div class="grid place-items-center py-20"><Spinner class="size-5" /></div>
		{:else}
			{@render children()}
		{/if}
	</div>

	{#if termMounted && ctx.ws}
		<div class="shrink-0 border-t border-border-strong bg-[var(--term-bg)] {termOpen ? '' : 'hidden'}" style="height: {termHeight}px">
			<div
				role="separator"
				aria-orientation="horizontal"
				class="h-1 cursor-row-resize bg-transparent hover:bg-primary/40"
				onpointerdown={startDrag}
			></div>
			<div class="flex h-8 items-center justify-between border-b border-white/5 px-3">
				<span class="inline-flex items-center gap-2 text-[12px] font-medium text-[#9aa3b5]">
					<SquareTerminal class="size-3.5" />Terminal
					<span class="mono font-normal text-[#697285]">{ctx.ws.resolved_dir}</span>
				</span>
				<div class="flex items-center">
					<button class="grid size-6 place-items-center rounded text-[#697285] hover:bg-white/5 hover:text-white" title="Hide" onclick={() => (termOpen = false)}><Minus class="size-3.5" /></button>
					<button
						class="grid size-6 place-items-center rounded text-[#697285] hover:bg-white/5 hover:text-white"
						title="Close session"
						onclick={() => {
							termOpen = false;
							termMounted = false;
						}}><X class="size-3.5" /></button
					>
				</div>
			</div>
			<div class="h-[calc(100%-36px)]">
				<Terminal
					bind:this={term}
					workspaceId={ctx.ws.id}
					workspaceName={ctx.ws.name}
					tool={ctx.ws.tool}
					onCommandStarted={() => ctx.reload()}
					onCommandFinished={() => ctx.reload()}
				/>
			</div>
		</div>
	{/if}
</div>
