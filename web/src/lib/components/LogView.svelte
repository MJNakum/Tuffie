<script lang="ts">
	import type { RunEvent } from '#lib/types';
	import { tick } from 'svelte';

	let { events, follow = true, maxHeight = '520px' }: { events: RunEvent[]; follow?: boolean; maxHeight?: string } = $props();
	let el: HTMLDivElement | undefined = $state();
	let stick = $state(true);

	$effect(() => {
		events.length;
		if (follow && stick && el) tick().then(() => el && (el.scrollTop = el.scrollHeight));
	});

	function onScroll() {
		if (!el) return;
		stick = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
	}
	const time = (ts: string) => new Date(ts).toLocaleTimeString(undefined, { hour12: false });
	const tone = (e: RunEvent) =>
		e.level === 'error' ? 'text-danger' : e.level === 'warn' ? 'text-warning' : e.type === 'command' ? 'text-primary-text' : e.type === 'change_summary' ? 'text-fg font-semibold' : 'text-[color-mix(in_srgb,var(--fg),var(--fg-muted)_35%)]';
</script>

<div
	bind:this={el}
	onscroll={onScroll}
	class="overflow-auto bg-[var(--term-bg)] py-2 font-mono text-[12px] leading-[1.65]"
	style="max-height: {maxHeight}"
>
	{#each events.filter((e) => e.type !== 'status') as e (e.seq)}
		<div class="group flex gap-3 px-4 hover:bg-white/[0.03]">
			<span class="w-16 shrink-0 select-none text-[#4b5263] tabular">{time(e.ts)}</span>
			<span class="min-w-0 flex-1 whitespace-pre-wrap break-words {tone(e)}">{e.message}{#if e.type === 'diagnostic' && e.data?.diagnostic?.detail}
					<span class="block text-[#8a93a6]">{e.data.diagnostic.detail}</span>{/if}</span>
		</div>
	{:else}
		<div class="px-4 py-2 text-[#5d6578]">Waiting for output…</div>
	{/each}
</div>
