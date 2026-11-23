import { getContext, setContext } from 'svelte';
import { api } from './api';
import type { WorkspaceDetail } from './types';

export class WorkspaceCtx {
	ws = $state<WorkspaceDetail | null>(null);
	error = $state('');
	/** Bumped when something changes, so tabs can reload. */
	version = $state(0);

	constructor(public id: string) {}

	async reload() {
		try {
			this.ws = await api.workspace(this.id);
			this.error = '';
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
		}
		this.version++;
	}
}

const KEY = Symbol('workspace');
export const setWorkspace = (c: WorkspaceCtx) => setContext(KEY, c);
export const useWorkspace = () => getContext<WorkspaceCtx>(KEY);
