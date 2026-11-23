<script lang="ts">
	import { api } from '#lib/api';
	import { useWorkspace } from '#lib/workspace.svelte';
	import type { Variable, VariableSet } from '#lib/types';
	import VariableEditor from '#lib/components/VariableEditor.svelte';
	import Spinner from '#lib/components/ui/Spinner.svelte';
	import Badge from '#lib/components/ui/Badge.svelte';
	import ShieldCheck from '@lucide/svelte/icons/shield-check';
	import Layers from '@lucide/svelte/icons/layers';
	import ArrowRight from '@lucide/svelte/icons/arrow-right';

	const ctx = useWorkspace();
	let vars = $state<Variable[] | null>(null);
	let sets = $state<VariableSet[]>([]);

	const load = async () => {
		[vars, sets] = await Promise.all([api.variables(ctx.id).catch(() => vars ?? []), api.workspaceVariableSets(ctx.id).catch(() => [])]);
	};
	$effect(() => {
		load();
	});

	const own = $derived(new Set((vars ?? []).map((v) => `${v.category}:${v.key}`)));
	// A set variable is overridden by the workspace, or by a later set in the list.
	function overriddenIn(i: number): Set<string> {
		const later = new Set(own);
		for (const s of sets.slice(i + 1)) for (const v of s.variables) later.add(`${v.category}:${v.key}`);
		return later;
	}
</script>

<div class="flex flex-col gap-5 px-8 py-6">
	<div class="flex items-start gap-3 rounded-lg border border-border bg-surface-2 px-4 py-3 text-[12.5px] text-muted">
		<ShieldCheck class="mt-0.5 size-4 shrink-0 text-primary-text" />
		<p>
			Every value is encrypted at rest with XChaCha20-Poly1305. Sensitive values are write-only: Tuffie never shows them again and
			replaces them with <code>***</code> in run logs and terminal output.
		</p>
	</div>
	{#if vars === null}
		<div class="grid place-items-center py-16"><Spinner /></div>
	{:else}
		<VariableEditor variables={vars} create={(b) => api.createVariable(ctx.id, b)} reload={load} />

		<section class="overflow-hidden rounded-lg border border-border bg-surface">
			<header class="flex items-center justify-between gap-4 border-b border-border px-4 py-3">
				<div>
					<h3 class="flex items-center gap-2 text-[13px] font-semibold"><Layers class="size-4 text-muted" />Inherited from variable sets</h3>
					<p class="text-[12px] text-muted">Applied before workspace variables. A workspace variable with the same key wins.</p>
				</div>
				<a href="/variable-sets" class="inline-flex items-center gap-1 text-[12.5px] text-primary-text hover:underline">Manage sets<ArrowRight class="size-3.5" /></a>
			</header>
			{#if sets.length === 0}
				<p class="px-4 py-6 text-center text-[12.5px] text-subtle">No variable set applies to this workspace.</p>
			{:else}
				<div class="divide-y divide-border">
					{#each sets as s, i (s.id)}
						{@const over = overriddenIn(i)}
						<div class="px-4 py-3">
							<div class="mb-2 flex items-center gap-2">
								<a href="/variable-sets/{s.id}" class="text-[13px] font-medium hover:text-primary-text">{s.name}</a>
								<Badge tone={s.global ? 'primary' : 'neutral'}>{s.global ? 'global' : 'assigned'}</Badge>
								<span class="text-[12px] text-subtle">{s.variables.length} variable{s.variables.length === 1 ? '' : 's'}</span>
							</div>
							<div class="flex flex-wrap gap-1.5">
								{#each s.variables as v (v.id)}
									{@const isOver = over.has(`${v.category}:${v.key}`)}
									<span
										class="mono inline-flex items-center gap-1 rounded border border-border-strong px-1.5 py-0.5 text-[11.5px] {isOver ? 'text-subtle line-through' : ''}"
										title={isOver ? 'Overridden by a later set or the workspace' : v.category === 'env' ? 'Environment variable' : 'Terraform variable'}
									>
										<span class="text-subtle">{v.category === 'env' ? '$' : 'var.'}</span>{v.key}
									</span>
								{/each}
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</section>
	{/if}
</div>
