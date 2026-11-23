<script lang="ts">
	import '../app.css';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { Toaster } from 'svelte-sonner';
	import { Tooltip } from 'bits-ui';
	import { app } from '#lib/app.svelte';
	import Logo from '#lib/components/Logo.svelte';
	import Boxes from '@lucide/svelte/icons/boxes';
	import Settings from '@lucide/svelte/icons/settings';
	import Layers from '@lucide/svelte/icons/layers';
	import ShieldCheck from '@lucide/svelte/icons/shield-check';
	import BookOpen from '@lucide/svelte/icons/book-open';
	import CircleHelp from '@lucide/svelte/icons/circle-help';
	import Compass from '@lucide/svelte/icons/compass';
	import Keyboard from '@lucide/svelte/icons/keyboard';
	import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import Menu from '#lib/components/ui/Menu.svelte';
	import { resetTours, startTour, tourForPath, tourList } from '#lib/tour';
	import Sun from '@lucide/svelte/icons/sun';
	import Moon from '@lucide/svelte/icons/moon';

	let { children } = $props();
	onMount(() => app.load());

	let helpOpen = $state(false);
	const contextTour = $derived(tourForPath(page.url.pathname));
	const helpItems = $derived([
		{ label: 'Tour of this page', hint: tourList.find((t) => t.name === contextTour)?.label, icon: Compass, onSelect: () => startTour(contextTour, true) },
		...tourList
			.filter((t) => t.name !== contextTour && t.name === 'welcome')
			.map((t) => ({ label: t.label, hint: 'Start from the workspace list', icon: Compass, onSelect: () => (goto('/'), setTimeout(() => startTour('welcome', true), 500)) })),
		'separator' as const,
		{ label: 'Documentation', hint: 'User guide and how-tos', icon: BookOpen, onSelect: () => goto('/docs') },
		{ label: 'Keyboard shortcuts', icon: Keyboard, onSelect: () => goto('/docs/shortcuts') },
		'separator' as const,
		{ label: 'Reset tours', hint: 'Show each tour again on first visit', icon: RotateCcw, onSelect: () => (resetTours(), toast.success('Tours reset')) }
	]);

	function onKey(e: KeyboardEvent) {
		const t = e.target as HTMLElement;
		const typing = t?.closest('input, textarea, [contenteditable="true"], .cm-editor, .xterm');
		if (e.key === '?' && !typing && !e.ctrlKey && !e.metaKey) {
			e.preventDefault();
			helpOpen = true;
		}
	}

	const nav = [
		{ href: '/', label: 'Workspaces', icon: Boxes, tour: 'nav-workspaces', match: (p: string) => p === '/' || p.startsWith('/workspaces') },
		{ href: '/variable-sets', label: 'Variable sets', icon: Layers, tour: 'nav-varsets', match: (p: string) => p.startsWith('/variable-sets') },
		{ href: '/policies', label: 'Policies', icon: ShieldCheck, tour: 'nav-policies', match: (p: string) => p.startsWith('/policies') },
		{ href: '/docs', label: 'Docs', icon: BookOpen, tour: 'nav-docs', match: (p: string) => p.startsWith('/docs') },
		{ href: '/settings', label: 'Settings', icon: Settings, tour: 'nav-settings', match: (p: string) => p.startsWith('/settings') }
	];
</script>

<svelte:window onkeydown={onKey} />

<Tooltip.Provider delayDuration={300}>
	<div class="flex h-screen overflow-hidden">
		<aside class="flex w-[220px] shrink-0 flex-col border-r border-border bg-surface">
			<div class="flex h-13 items-center px-4"><a href="/"><Logo /></a></div>
			<nav class="flex flex-col gap-0.5 px-2 pt-2">
				{#each nav as n (n.href)}
					{@const active = n.match(page.url.pathname)}
					<a
						href={n.href}
						data-tour={n.tour}
						class="flex h-8 items-center gap-2.5 rounded-md px-2.5 text-[13px] font-medium transition-colors {active
							? 'bg-surface-3 text-fg'
							: 'text-muted hover:bg-hover hover:text-fg'}"
					>
						<n.icon class="size-4 {active ? 'text-primary-text' : ''}" />
						{n.label}
					</a>
				{/each}
			</nav>
			<div class="mt-auto flex flex-col gap-2 border-t border-border p-3">
				<div class="flex items-center justify-between">
					<span class="inline-flex items-center gap-1.5 text-[12px] text-muted" data-tour="mode">
						<span class="size-1.5 rounded-full {app.system ? 'bg-success' : 'bg-danger'}"></span>
						{app.system ? (app.system.mode === 'local' ? 'Local mode' : 'Server mode') : 'Offline'}
					</span>
					<div class="flex items-center">
						<Menu items={helpItems} bind:open={helpOpen} side="right" align="end">
							{#snippet trigger(props)}
								<button
									{...props}
									data-tour="help"
									class="grid size-7 place-items-center rounded-md text-subtle hover:bg-hover hover:text-fg data-[state=open]:bg-hover data-[state=open]:text-fg"
									title="Help (?)"><CircleHelp class="size-3.5" /></button
								>
							{/snippet}
						</Menu>
						<button
							class="grid size-7 place-items-center rounded-md text-subtle hover:bg-hover hover:text-fg"
							onclick={() => app.toggleTheme()}
							title="Switch theme"
						>
							{#if app.theme === 'dark'}<Sun class="size-3.5" />{:else}<Moon class="size-3.5" />{/if}
						</button>
					</div>
				</div>
				{#if app.system}<span class="text-[11px] text-subtle">v{app.system.version}</span>{/if}
			</div>
		</aside>
		<main class="flex min-w-0 flex-1 flex-col overflow-hidden">
			{@render children()}
		</main>
	</div>
</Tooltip.Provider>
<Toaster
	theme={app.theme}
	position="bottom-right"
	toastOptions={{ class: '!bg-surface-2 !border-border-strong !text-fg !text-[13px] !font-sans' }}
/>
