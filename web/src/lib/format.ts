import type { Action, RunKind, RunStatus } from './types';

export function ago(iso: string | null | undefined): string {
	if (!iso) return '—';
	const s = Math.round((Date.now() - new Date(iso).getTime()) / 1000);
	if (s < 5) return 'just now';
	if (s < 60) return `${s}s ago`;
	const m = Math.round(s / 60);
	if (m < 60) return `${m}m ago`;
	const h = Math.round(m / 60);
	if (h < 24) return `${h}h ago`;
	const d = Math.round(h / 24);
	if (d < 30) return `${d}d ago`;
	return new Date(iso).toLocaleDateString();
}

export function dateTime(iso: string | null | undefined): string {
	if (!iso) return '—';
	return new Date(iso).toLocaleString(undefined, {
		month: 'short',
		day: 'numeric',
		hour: '2-digit',
		minute: '2-digit',
		second: '2-digit'
	});
}

export function duration(from: string | null, to: string | null): string {
	if (!from) return '';
	const ms = (to ? new Date(to).getTime() : Date.now()) - new Date(from).getTime();
	const s = Math.max(0, Math.round(ms / 1000));
	if (s < 60) return `${s}s`;
	const m = Math.floor(s / 60);
	return `${m}m ${s % 60}s`;
}

export const kindLabel: Record<RunKind, string> = {
	plan_only: 'Plan only',
	plan_and_apply: 'Plan and apply',
	destroy: 'Destroy',
	refresh: 'Refresh state',
	drift: 'Drift check',
	cli: 'Terminal'
};

export type Tone = 'neutral' | 'primary' | 'success' | 'warning' | 'danger' | 'violet';

export const statusInfo: Record<RunStatus, { label: string; tone: Tone; active?: boolean }> = {
	pending: { label: 'Pending', tone: 'neutral' },
	planning: { label: 'Planning', tone: 'primary', active: true },
	planned: { label: 'Needs confirmation', tone: 'warning' },
	planned_and_finished: { label: 'Planned', tone: 'success' },
	applying: { label: 'Applying', tone: 'primary', active: true },
	applied: { label: 'Applied', tone: 'success' },
	discarded: { label: 'Discarded', tone: 'neutral' },
	canceled: { label: 'Canceled', tone: 'neutral' },
	errored: { label: 'Errored', tone: 'danger' },
	policy_failed: { label: 'Policy failed', tone: 'danger' },
	running: { label: 'Running', tone: 'primary', active: true },
	succeeded: { label: 'Succeeded', tone: 'success' },
	failed: { label: 'Failed', tone: 'danger' }
};

export function isActive(s: RunStatus): boolean {
	return ['pending', 'planning', 'applying', 'running'].includes(s);
}

/** Collapse Terraform's action list into one display action. */
export function actionOf(actions: Action[]): 'create' | 'update' | 'delete' | 'replace' | 'read' | 'no-op' {
	if (actions.includes('delete') && actions.includes('create')) return 'replace';
	if (actions.includes('create')) return 'create';
	if (actions.includes('delete')) return 'delete';
	if (actions.includes('update')) return 'update';
	if (actions.includes('read')) return 'read';
	return 'no-op';
}

export const actionInfo = {
	create: { sym: '+', label: 'create', tone: 'success' as Tone },
	update: { sym: '~', label: 'update', tone: 'warning' as Tone },
	delete: { sym: '−', label: 'destroy', tone: 'danger' as Tone },
	replace: { sym: '±', label: 'replace', tone: 'violet' as Tone },
	read: { sym: '≤', label: 'read', tone: 'primary' as Tone },
	'no-op': { sym: '·', label: 'no change', tone: 'neutral' as Tone }
};

export const toneText: Record<Tone, string> = {
	neutral: 'text-muted',
	primary: 'text-primary-text',
	success: 'text-success',
	warning: 'text-warning',
	danger: 'text-danger',
	violet: 'text-violet'
};

export const toneBg: Record<Tone, string> = {
	neutral: 'bg-surface-3 text-muted',
	primary: 'bg-primary-soft text-primary-text',
	success: 'bg-success-soft text-success',
	warning: 'bg-warning-soft text-warning',
	danger: 'bg-danger-soft text-danger',
	violet: 'bg-violet-soft text-violet'
};

export function shortPath(p: string | null | undefined, max = 48): string {
	if (!p) return '';
	if (p.length <= max) return p;
	return '…' + p.slice(p.length - max + 1);
}

export const driftIntervals = [
	{ value: '0', label: 'Off' },
	{ value: '60', label: 'Every hour' },
	{ value: '360', label: 'Every 6 hours' },
	{ value: '720', label: 'Every 12 hours' },
	{ value: '1440', label: 'Every day' },
	{ value: '10080', label: 'Every week' }
];

export function intervalLabel(min: number): string {
	return driftIntervals.find((d) => Number(d.value) === min)?.label ?? `Every ${min} minutes`;
}

export const policyTone: Record<string, Tone> = { passed: 'success', warned: 'warning', failed: 'danger', errored: 'danger' };
