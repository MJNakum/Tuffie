<script lang="ts">
	let { add = 0, change = 0, destroy = 0, size = 'sm' }: { add?: number; change?: number; destroy?: number; size?: 'sm' | 'lg' } =
		$props();
	const cls = $derived(size === 'lg' ? 'text-[13px] gap-3' : 'text-[12px] gap-2');
</script>

<span class="mono inline-flex items-center tabular {cls}">
	<span class={add ? 'text-success' : 'text-subtle'} title="{add} to add">+{add}</span>
	<span class={change ? 'text-warning' : 'text-subtle'} title="{change} to change">~{change}</span>
	<span class={destroy ? 'text-danger' : 'text-subtle'} title="{destroy} to destroy">−{destroy}</span>
</span>
