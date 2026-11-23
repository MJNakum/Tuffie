<script lang="ts">
	let { class: cls = 'size-4' }: { class?: string } = $props();
</script>

<svg class="animate-spin text-muted {cls}" viewBox="0 0 24 24" fill="none"
	><circle cx="12" cy="12" r="9" stroke="currentColor" stroke-opacity=".2" stroke-width="2.5" /><path
		d="M21 12a9 9 0 0 0-9-9"
		stroke="currentColor"
		stroke-width="2.5"
		stroke-linecap="round"
	/></svg
>
