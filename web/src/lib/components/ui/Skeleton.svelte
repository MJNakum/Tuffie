<script lang="ts">
	// A placeholder with the size of the content that will replace it,
	// so nothing moves when the data arrives.
	let { class: cls = 'h-3 w-24', style = '' }: { class?: string; style?: string } = $props();
</script>

<span class="skeleton block {cls}" {style} aria-hidden="true"></span>
