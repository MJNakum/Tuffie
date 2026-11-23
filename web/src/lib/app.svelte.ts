import { untrack } from 'svelte';
import { api } from './api';
import type { SystemInfo } from './types';

class AppState {
	system = $state<SystemInfo | null>(null);
	theme = $state<'dark' | 'light'>('dark');

	async load() {
		try {
			this.system = await api.system();
		} catch {
			/* server offline: the layout shows a banner */
		}
		const t = document.documentElement.dataset.theme;
		this.theme = t === 'light' ? 'light' : 'dark';
	}

	toggleTheme() {
		this.theme = this.theme === 'dark' ? 'light' : 'dark';
		document.documentElement.dataset.theme = this.theme;
		try {
			localStorage.setItem('tuffie-theme', this.theme);
		} catch {
			/* storage unavailable */
		}
	}
}

export const app = new AppState();

/**
 * Run a function now and then every `ms` while the page is visible.
 * The first call is untracked, so state it reads does not become a
 * dependency of the calling effect (which would restart it in a loop).
 */
export function poll(fn: () => unknown, ms: number) {
	untrack(fn);
	const id = setInterval(() => {
		if (document.visibilityState === 'visible') fn();
	}, ms);
	return () => clearInterval(id);
}
