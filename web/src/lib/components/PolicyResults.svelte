<script lang="ts">
	import { policyTone, toneBg, toneText } from '#lib/format';
	import type { PolicySetResult } from '#lib/types';
	import Badge from './ui/Badge.svelte';
	import CircleCheck from '@lucide/svelte/icons/circle-check';
	import CircleX from '@lucide/svelte/icons/circle-x';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';

	let { sets }: { sets: PolicySetResult[] } = $props();
	const label = { passed: 'Passed', warned: 'Warnings', failed: 'Failed', errored: 'Error' };
</script>

<div class="motion-rise divide-y divide-border">
	{#each sets as s (s.set_id)}
		<div class="px-4 py-3">
			<div class="flex items-center gap-2.5">
				<span class="inline-flex h-[22px] items-center gap-1 rounded-full px-2 text-2xs font-medium {toneBg[policyTone[s.status]]}">
					{#if s.status === 'passed'}<CircleCheck class="size-3.5" />{:else if s.status === 'warned'}<TriangleAlert class="size-3.5" />{:else}<CircleX class="size-3.5" />{/if}
					{label[s.status]}
				</span>
				<a href="/policies/{s.set_id}" class="text-base font-medium hover:text-primary-text">{s.set_name}</a>
				<Badge tone={s.enforcement === 'mandatory' ? 'danger' : 'neutral'}>{s.enforcement}</Badge>
				<span class="ml-auto text-xs text-subtle">{s.policy_count} polic{s.policy_count === 1 ? 'y' : 'ies'}</span>
			</div>
			{#if s.error}
				<pre class="mt-2 whitespace-pre-wrap rounded-md bg-danger-soft px-3 py-2 font-mono text-xs text-danger">{s.error}</pre>
			{/if}
			{#if s.findings.length}
				<ul class="mt-2 flex flex-col gap-1">
					{#each s.findings as f, i (i)}
						{@const blocking = f.level === 'fail' && s.enforcement === 'mandatory'}
						<li class="flex items-start gap-2 text-sm">
							<span class="mt-[3px] size-2 shrink-0 rounded-full {blocking ? 'bg-danger' : 'bg-warning'}"></span>
							<span class="min-w-0 flex-1">
								{f.message}
								<span class="mono ml-1 text-2xs text-subtle">{f.package}.{f.rule}</span>
								{#if f.details}<span class="mono mt-0.5 block text-2xs text-subtle">{JSON.stringify(f.details)}</span>{/if}
							</span>
							<span class="text-2xs font-medium {blocking ? toneText.danger : toneText.warning}">{f.level === 'fail' ? (blocking ? 'blocks apply' : 'advisory') : 'warning'}</span>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	{/each}
</div>
