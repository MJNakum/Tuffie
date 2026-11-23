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
	import Sun from '@lucide/svelte/icons/sun';
	import Moon from '@lucide/svelte/icons/moon';

	let { children } = $props();
	onMount(() => app.load());

	const nav = [
		{ href: '/', label: 'Workspaces', icon: Boxes, match: (p: string) => p === '/' || p.startsWith('/workspaces') },
		{ href: '/variable-sets', label: 'Variable sets', icon: Layers, match: (p: string) => p.startsWith('/variable-sets') },
		{ href: '/policies', label: 'Policies', icon: ShieldCheck, match: (p: string) => p.startsWith('/policies') },
		{ href: '/settings', label: 'Settings', icon: Settings, match: (p: string) => p.startsWith('/settings') }
	];
</script>

<Tooltip.Provider delayDuration={300}>
	<div class="flex h-screen overflow-hidden">
		<aside class="flex w-[220px] shrink-0 flex-col border-r border-border bg-surface">
			<div class="flex h-13 items-center px-4"><a href="/"><Logo /></a></div>
			<nav class="flex flex-col gap-0.5 px-2 pt-2">
				{#each nav as n (n.href)}
					{@const active = n.match(page.url.pathname)}
					<a
						href={n.href}
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
					<span class="inline-flex items-center gap-1.5 text-[12px] text-muted">
						<span class="size-1.5 rounded-full {app.system ? 'bg-success' : 'bg-danger'}"></span>
						{app.system ? (app.system.mode === 'local' ? 'Local mode' : 'Server mode') : 'Offline'}
					</span>
					<div class="flex items-center">
						<a
							href="https://github.com/MJNakum/Tuffie#readme"
							target="_blank"
							rel="noreferrer"
							class="grid size-7 place-items-center rounded-md text-subtle hover:bg-hover hover:text-fg"
							title="Documentation"><BookOpen class="size-3.5" /></a
						>
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
