<script lang="ts" module>
	export type MenuItem =
		| { label: string; hint?: string; icon?: any; danger?: boolean; onSelect: () => void; disabled?: boolean }
		| 'separator';
</script>

<script lang="ts">
	import { DropdownMenu } from 'bits-ui';
	import type { Snippet } from 'svelte';
	let { items, trigger, align = 'end' }: { items: MenuItem[]; trigger: Snippet<[Record<string, unknown>]>; align?: 'start' | 'end' } =
		$props();
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger>
		{#snippet child({ props })}
			{@render trigger(props)}
		{/snippet}
	</DropdownMenu.Trigger>
	<DropdownMenu.Portal>
		<DropdownMenu.Content
			{align}
			sideOffset={6}
			class="z-50 min-w-56 rounded-lg border border-border-strong bg-surface-2 p-1 shadow-pop focus:outline-none"
		>
			{#each items as it, i (i)}
				{#if it === 'separator'}
					<DropdownMenu.Separator class="my-1 h-px bg-border" />
				{:else}
					<DropdownMenu.Item
						disabled={it.disabled}
						onSelect={it.onSelect}
						class="flex cursor-pointer items-start gap-2.5 rounded-md px-2 py-1.5 text-[13px] outline-none data-[disabled]:opacity-50 data-[highlighted]:bg-hover {it.danger
							? 'text-danger'
							: 'text-fg'}"
					>
						{#if it.icon}
							<it.icon class="mt-0.5 size-4 shrink-0 {it.danger ? '' : 'text-muted'}" />
						{/if}
						<div class="flex flex-col">
							<span>{it.label}</span>
							{#if it.hint}<span class="text-[11.5px] text-subtle">{it.hint}</span>{/if}
						</div>
					</DropdownMenu.Item>
				{/if}
			{/each}
		</DropdownMenu.Content>
	</DropdownMenu.Portal>
</DropdownMenu.Root>
