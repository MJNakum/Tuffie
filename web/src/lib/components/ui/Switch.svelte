<script lang="ts">
	import { Switch } from 'bits-ui';
	let { checked = $bindable(false), id, disabled = false }: { checked?: boolean; id?: string; disabled?: boolean } = $props();
</script>

<Switch.Root
	bind:checked
	{id}
	{disabled}
	class="peer inline-flex h-[18px] w-8 shrink-0 cursor-pointer items-center rounded-full border border-border-strong bg-surface-3 px-[2px] transition-colors data-[state=checked]:border-primary data-[state=checked]:bg-primary disabled:opacity-50"
>
	<Switch.Thumb
		class="pointer-events-none block size-3 rounded-full bg-fg shadow transition-transform data-[state=checked]:translate-x-[14px] data-[state=checked]:bg-white"
	/>
</Switch.Root>
