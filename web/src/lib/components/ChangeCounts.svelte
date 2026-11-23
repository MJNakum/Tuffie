<script lang="ts">
	let { add = 0, change = 0, destroy = 0, size = 'sm' }: { add?: number; change?: number; destroy?: number; size?: 'sm' | 'lg' } =
		$props();
	const cls = $derived(size === 'lg' ? 'text-base gap-3' : 'text-xs gap-2');
</script>

<span class="mono inline-flex items-center tabular {cls}">
	<span class="transition-colors duration-(--dur-slow) {add ? 'text-success' : 'text-subtle'}" title="{add} to add">+{add}</span>
	<span class="transition-colors duration-(--dur-slow) {change ? 'text-warning' : 'text-subtle'}" title="{change} to change">~{change}</span>
	<span class="transition-colors duration-(--dur-slow) {destroy ? 'text-danger' : 'text-subtle'}" title="{destroy} to destroy">−{destroy}</span>
</span>
