<script lang="ts">
	import type { Snippet } from 'svelte';
	import { toneBg, type Tone } from '#lib/format';
	let { tone = 'neutral', class: cls = '', children }: { tone?: Tone; class?: string; children: Snippet } = $props();
</script>

<span class="inline-flex h-5 items-center gap-1 rounded px-1.5 text-[11.5px] font-medium {toneBg[tone]} {cls}">
	{@render children()}
</span>
