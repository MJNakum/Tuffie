<script lang="ts">
	import type { HTMLTextareaAttributes } from 'svelte/elements';
	let { value = $bindable(), class: cls = '', mono = false, ...rest }: HTMLTextareaAttributes & { mono?: boolean } = $props();
</script>

<textarea
	bind:value
	class="min-h-20 w-full rounded-md border border-border-strong bg-surface px-2.5 py-2 text-base text-fg placeholder:text-subtle focus:border-primary focus:outline-none focus:ring-3 focus:ring-primary-soft transition-[border-color,box-shadow] duration-(--dur-fast) {mono
		? 'font-mono text-sm'
		: ''} {cls}"
	{...rest}
></textarea>
