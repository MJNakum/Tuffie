<script lang="ts">
	// Dependency graph. Columns are dependency depth: a resource sits to the
	// right of everything it depends on.
	import type { StateView } from '#lib/types';

	let { graph, selected = null, onSelect }: { graph: StateView['graph']; selected?: string | null; onSelect?: (id: string) => void } =
		$props();

	const W = 220;
	const H = 40;
	const GX = 70;
	const GY = 14;

	const layout = $derived.by(() => {
		const ids = new Set(graph.nodes.map((n) => n.id));
		const deps = new Map<string, string[]>();
		for (const n of graph.nodes) deps.set(n.id, []);
		// Edges point from a resource to what it depends on. depends_on can name a
		// resource without an index, so match by prefix too.
		for (const e of graph.edges) {
			const targets = ids.has(e.to) ? [e.to] : [...ids].filter((i) => i.startsWith(e.to + '['));
			for (const t of targets) if (t !== e.from) deps.get(e.from)?.push(t);
		}
		const depth = new Map<string, number>();
		const visiting = new Set<string>();
		const d = (id: string): number => {
			if (depth.has(id)) return depth.get(id)!;
			if (visiting.has(id)) return 0;
			visiting.add(id);
			const v = Math.max(-1, ...(deps.get(id) ?? []).map(d)) + 1;
			visiting.delete(id);
			depth.set(id, v);
			return v;
		};
		graph.nodes.forEach((n) => d(n.id));
		const cols: string[][] = [];
		for (const n of graph.nodes) (cols[depth.get(n.id)!] ??= []).push(n.id);
		const pos = new Map<string, { x: number; y: number }>();
		cols.forEach((col, ci) => col.sort().forEach((id, ri) => pos.set(id, { x: 16 + ci * (W + GX), y: 16 + ri * (H + GY) })));
		const edges: { from: string; to: string; path: string }[] = [];
		for (const [from, ts] of deps) {
			for (const to of ts) {
				const a = pos.get(to)!;
				const b = pos.get(from)!;
				const x1 = a.x + W,
					y1 = a.y + H / 2,
					x2 = b.x,
					y2 = b.y + H / 2;
				const mx = (x1 + x2) / 2;
				edges.push({ from, to, path: `M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}` });
			}
		}
		const width = 32 + cols.length * (W + GX) - GX;
		const height = 32 + Math.max(1, ...cols.map((c) => c.length)) * (H + GY) - GY;
		return { pos, edges, width, height, types: new Map(graph.nodes.map((n) => [n.id, n])) };
	});

	const related = $derived.by(() => {
		if (!selected) return new Set<string>();
		const s = new Set<string>([selected]);
		for (const e of layout.edges) {
			if (e.from === selected) s.add(e.to);
			if (e.to === selected) s.add(e.from);
		}
		return s;
	});
</script>

<div class="h-full overflow-auto bg-[radial-gradient(var(--border)_1px,transparent_1px)] [background-size:18px_18px]">
	<svg width={layout.width} height={layout.height} class="block">
		<defs>
			<marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
				<path d="M 0 0 L 10 5 L 0 10 z" fill="var(--border-strong)" />
			</marker>
			<marker id="arrow-hi" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
				<path d="M 0 0 L 10 5 L 0 10 z" fill="var(--primary)" />
			</marker>
		</defs>
		{#each layout.edges as e (e.from + '>' + e.to)}
			{@const hi = selected && (e.from === selected || e.to === selected)}
			<path d={e.path} fill="none" stroke={hi ? 'var(--primary)' : 'var(--border-strong)'} stroke-width={hi ? 1.6 : 1.2} marker-end={hi ? 'url(#arrow-hi)' : 'url(#arrow)'} />
		{/each}
		{#each [...layout.pos] as [id, p] (id)}
			{@const n = layout.types.get(id)}
			{@const dim = selected && !related.has(id)}
			<g
				transform="translate({p.x},{p.y})"
				class="cursor-pointer"
				opacity={dim ? 0.35 : 1}
				role="button"
				tabindex="0"
				onclick={() => onSelect?.(id)}
				onkeydown={(ev) => ev.key === 'Enter' && onSelect?.(id)}
			>
				<rect width={W} height={H} rx="7" fill="var(--surface)" stroke={selected === id ? 'var(--primary)' : 'var(--border-strong)'} stroke-width={selected === id ? 1.5 : 1} />
				<rect x="0" y="0" width="3" height={H} rx="1.5" fill={n?.mode === 'data' ? 'var(--violet)' : 'var(--primary)'} />
				<text x="12" y="16" font-size="10.5" fill="var(--fg-subtle)" font-family="var(--font-mono)">{(n?.mode === 'data' ? 'data.' : '') + (n?.type ?? '')}</text>
				<text x="12" y="31" font-size="12" fill="var(--fg)" font-family="var(--font-mono)">{id.length > 28 ? '…' + id.slice(-27) : id}</text>
			</g>
		{/each}
	</svg>
</div>
