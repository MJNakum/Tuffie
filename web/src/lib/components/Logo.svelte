<script lang="ts">
	let { class: cls = '' }: { class?: string } = $props();
</script>

<div class="flex items-center gap-2 {cls}">
	<svg viewBox="0 0 32 32" class="size-6" aria-hidden="true">
		<rect width="32" height="32" rx="7" fill="var(--surface-3)" />
		<path d="M9 10h14v3.2h-5.4V23h-3.2v-9.8H9z" fill="var(--primary)" />
		<rect x="19.6" y="17.6" width="3.4" height="3.4" rx=".8" fill="var(--primary-text)" />
	</svg>
	<span class="text-lg font-semibold tracking-[-0.02em]">tuffie</span>
</div>
