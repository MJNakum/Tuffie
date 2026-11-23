<script lang="ts">
	import { Dialog } from 'bits-ui';
	import X from '@lucide/svelte/icons/x';
	import type { Snippet } from 'svelte';
	let {
		open = $bindable(false),
		title,
		description,
		width = 'max-w-lg',
		children,
		footer
	}: { open?: boolean; title: string; description?: string; width?: string; children: Snippet; footer?: Snippet } = $props();
</script>

<Dialog.Root bind:open>
	<Dialog.Portal>
		<Dialog.Overlay class="fixed inset-0 z-50 bg-black/60 backdrop-blur-[2px] data-[state=open]:animate-in" />
		<Dialog.Content
			class="fixed left-1/2 top-[12vh] z-50 w-[calc(100vw-32px)] {width} -translate-x-1/2 rounded-xl border border-border-strong bg-surface shadow-pop focus:outline-none"
		>
			<div class="flex items-start justify-between gap-4 border-b border-border px-5 py-4">
				<div>
					<Dialog.Title class="text-[15px] font-semibold tracking-[-0.01em]">{title}</Dialog.Title>
					{#if description}
						<Dialog.Description class="mt-0.5 text-[12.5px] text-muted">{description}</Dialog.Description>
					{/if}
				</div>
				<Dialog.Close class="-mr-1.5 rounded-md p-1 text-subtle hover:bg-hover hover:text-fg"><X class="size-4" /></Dialog.Close>
			</div>
			<div class="px-5 py-4">{@render children()}</div>
			{#if footer}
				<div class="flex justify-end gap-2 rounded-b-xl border-t border-border bg-surface-2 px-5 py-3">{@render footer()}</div>
			{/if}
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>
