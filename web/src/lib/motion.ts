// Svelte transitions that follow the motion tokens in app.css.
// Every helper returns a zero duration when the OS asks for reduced motion.
import { prefersReducedMotion } from 'svelte/motion';
import { slide, fly, fade } from 'svelte/transition';
import { cubicIn, quintOut } from 'svelte/easing';
import { flip as svelteFlip } from 'svelte/animate';

export const DUR = { fast: 140, base: 200, slow: 260, page: 280 } as const;
const EXIT = 0.7;

const d = (ms: number) => (prefersReducedMotion.current ? 0 : ms);

type P = { duration?: number; delay?: number };

/** Expand / collapse: height and opacity together, so content never snaps. */
export function reveal(node: Element, p: P = {}) {
	const s = slide(node, { duration: d(p.duration ?? DUR.slow), delay: p.delay, easing: quintOut });
	return {
		...s,
		css: (t: number, u: number) => `${s.css?.(t, u) ?? ''};opacity:${Math.min(1, t * 1.6)};`
	};
}

/** Collapse (out) is a little faster than expand (in). */
export function conceal(node: Element, p: P = {}) {
	const s = slide(node, { duration: d((p.duration ?? DUR.slow) * EXIT), easing: cubicIn });
	return { ...s, css: (t: number, u: number) => `${s.css?.(t, u) ?? ''};opacity:${t};` };
}

/** A new item arrives: fade and rise a few pixels. */
export function rise(node: Element, p: P & { y?: number } = {}) {
	return fly(node, { y: prefersReducedMotion.current ? 0 : (p.y ?? 6), duration: d(p.duration ?? DUR.slow), delay: p.delay, easing: quintOut });
}

/** An item leaves: a short fade. */
export function vanish(node: Element, p: P = {}) {
	return fade(node, { duration: d((p.duration ?? DUR.base) * EXIT), easing: cubicIn });
}

export function fadeIn(node: Element, p: P = {}) {
	return fade(node, { duration: d(p.duration ?? DUR.base), delay: p.delay, easing: quintOut });
}

/** Re-ordered list items glide to their new place. */
export function flip(node: Element, rects: { from: DOMRect; to: DOMRect }, _p?: unknown) {
	return svelteFlip(node, rects, { duration: d(DUR.slow), easing: quintOut });
}

/**
 * Sliding indicator for tabs, segmented controls and navigation.
 * Put one `<span data-indicator>` inside the container. The action moves it
 * under the active item (`[data-active="true"]` or `[aria-current="page"]`).
 * `mode: 'underline'` draws a 2 px bar at the bottom instead of a fill.
 */
export function indicator(node: HTMLElement, opts: { mode?: 'fill' | 'underline' } = {}) {
	let mode = opts.mode ?? 'fill';
	let first = true;
	const bar = () => node.querySelector<HTMLElement>(':scope > [data-indicator]');

	function place() {
		const el = bar();
		if (!el) return;
		const active = node.querySelector<HTMLElement>('[data-active="true"], [aria-current="page"]');
		if (!active) {
			el.style.opacity = '0';
			return;
		}
		const r = active.getBoundingClientRect();
		const p = node.getBoundingClientRect();
		const x = r.left - p.left + node.scrollLeft;
		const y = mode === 'underline' ? p.height - 2 : r.top - p.top + node.scrollTop;
		const motion = prefersReducedMotion.current || first ? 'none' : 'var(--dur-base) var(--ease-move)';
		el.style.transition =
			motion === 'none' ? 'none' : `transform ${motion}, width ${motion}, height ${motion}, opacity var(--dur-fast) linear`;
		el.style.width = `${r.width}px`;
		el.style.height = mode === 'underline' ? '2px' : `${r.height}px`;
		el.style.transform = `translate(${x}px, ${y}px)`;
		el.style.opacity = '1';
		first = false;
	}

	const ro = new ResizeObserver(() => place());
	ro.observe(node);
	const mo = new MutationObserver(() => requestAnimationFrame(place));
	mo.observe(node, { subtree: true, childList: true, attributes: true, attributeFilter: ['data-active', 'aria-current'] });
	requestAnimationFrame(place);

	return {
		update(o: { mode?: 'fill' | 'underline' } = {}) {
			mode = o.mode ?? 'fill';
			place();
		},
		destroy() {
			ro.disconnect();
			mo.disconnect();
		}
	};
}
