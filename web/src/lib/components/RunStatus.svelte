<script lang="ts">
	import { statusInfo, toneBg, toneText } from '#lib/format';
	import type { RunStatus } from '#lib/types';
	import CircleCheck from '@lucide/svelte/icons/circle-check';
	import CircleX from '@lucide/svelte/icons/circle-x';
	import CircleDashed from '@lucide/svelte/icons/circle-dashed';
	import CircleSlash from '@lucide/svelte/icons/circle-slash';
	import CircleAlert from '@lucide/svelte/icons/circle-alert';
	import Clock from '@lucide/svelte/icons/clock';

	let {
		status,
		variant = 'badge',
		kind,
		hasChanges
	}: { status: RunStatus; variant?: 'badge' | 'icon' | 'text'; kind?: string; hasChanges?: boolean | null } = $props();
	const info = $derived(
		kind === 'drift' && status === 'planned_and_finished'
			? hasChanges
				? { label: 'Drift detected', tone: 'warning' as const, active: false }
				: { label: 'No drift', tone: 'success' as const, active: false }
			: (statusInfo[status] ?? { label: status, tone: 'neutral' })
	);
	const Icon = $derived(
		info.active
			? null
			: status === 'pending'
				? Clock
				: status === 'planned'
					? CircleAlert
					: info.tone === 'warning'
						? CircleAlert
						: info.tone === 'success'
						? CircleCheck
						: info.tone === 'danger'
							? CircleX
							: status === 'discarded' || status === 'canceled'
								? CircleSlash
								: CircleDashed
	);
</script>

{#snippet glyph(size: string)}
	{#if info.active}
		<span class="relative grid {size} place-items-center">
			<span class="absolute inset-[3px] rounded-full bg-current opacity-25 pulse-dot"></span>
			<span class="size-[45%] rounded-full bg-current"></span>
		</span>
	{:else if Icon}
		<Icon class={size} />
	{/if}
{/snippet}

{#if variant === 'badge'}
	<span class="inline-flex h-[22px] items-center gap-1.5 rounded-full pl-1.5 pr-2 text-[11.5px] font-medium {toneBg[info.tone]}">
		{@render glyph('size-3.5')}
		{info.label}
	</span>
{:else if variant === 'icon'}
	<span class={toneText[info.tone]} title={info.label}>{@render glyph('size-4')}</span>
{:else}
	<span class="inline-flex items-center gap-1.5 text-[12.5px] font-medium {toneText[info.tone]}">
		{@render glyph('size-3.5')}
		{info.label}
	</span>
{/if}
