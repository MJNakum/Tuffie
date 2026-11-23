<script lang="ts">
	import { app } from '#lib/app.svelte';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import Card from '#lib/components/ui/Card.svelte';
	import Badge from '#lib/components/ui/Badge.svelte';
	import Spinner from '#lib/components/ui/Spinner.svelte';
	import Sun from '@lucide/svelte/icons/sun';
	import Moon from '@lucide/svelte/icons/moon';
	import Check from '@lucide/svelte/icons/check';

	$effect(() => {
		app.load();
	});
	const s = $derived(app.system);
</script>

<PageHeader title="Settings" subtitle="How this Tuffie server runs. Start options are set with command-line flags or TUFFIE_* environment variables." />

<div class="min-h-0 flex-1 overflow-auto px-8 py-6">
	{#if !s}
		<div class="grid place-items-center py-16"><Spinner /></div>
	{:else}
		<div class="mx-auto flex max-w-3xl flex-col gap-5">
			<Card title="Server">
				<dl class="grid grid-cols-[180px_minmax(0,1fr)] gap-x-4 gap-y-3 text-[12.5px]">
					<dt class="text-subtle">Version</dt><dd class="mono">{s.version}</dd>
					<dt class="text-subtle">Mode</dt>
					<dd>
						<Badge tone={s.mode === 'local' ? 'primary' : 'warning'}>{s.mode}</Badge>
						<span class="ml-2 text-muted">{s.mode === 'local' ? 'Single user, no login, loopback only.' : 'Shared server.'}</span>
					</dd>
					<dt class="text-subtle">Data folder</dt><dd class="mono break-all">{s.data_dir}</dd>
					<dt class="text-subtle">Inherit environment</dt>
					<dd>{s.inherit_env ? 'Yes — runs see the same environment as your shell (AWS_PROFILE, ~/.aws, …).' : 'No — runs get a minimal environment plus workspace variables.'}</dd>
				</dl>
			</Card>
			<Card title="Detected tools" pad={false}>
				<ul class="divide-y divide-border">
					{#each s.tools as t (t.name)}
						<li class="flex items-center gap-4 px-4 py-3 text-[12.5px]">
							<span class="w-24 font-medium">{t.name === 'tofu' ? 'OpenTofu' : 'Terraform'}</span>
							{#if t.path}
								<span class="inline-flex items-center gap-1.5 text-success"><Check class="size-3.5" />v{t.version ?? '?'}</span>
								<span class="mono truncate text-muted">{t.path}</span>
							{:else}
								<span class="text-subtle">Not found on PATH</span>
							{/if}
						</li>
					{/each}
				</ul>
			</Card>
			<Card title="Appearance">
				<div class="grid grid-cols-2 gap-3">
					{#each [{ k: 'dark', l: 'Dark', I: Moon }, { k: 'light', l: 'Light', I: Sun }] as { k, l, I } (k)}
						<button
							class="flex items-center gap-2.5 rounded-lg border px-3.5 py-3 text-left text-[13px] font-medium {app.theme === k ? 'border-primary bg-primary-soft' : 'border-border-strong hover:bg-hover'}"
							onclick={() => app.theme !== k && app.toggleTheme()}
						>
							<I class="size-4" />{l}
						</button>
					{/each}
				</div>
			</Card>
		</div>
	{/if}
</div>
