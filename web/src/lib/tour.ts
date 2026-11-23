// Guided tours, built on driver.js. Each step points at an element with a
// `data-tour="…"` attribute. Steps whose element is not on the page are
// skipped, so a tour works on an empty install and on a busy one.
import { driver, type DriveStep, type Driver } from 'driver.js';
import 'driver.js/dist/driver.css';
import { goto } from '$app/navigation';
import { api } from './api';

export type TourName = 'welcome' | 'workspace' | 'run';

type Step = { el?: string; title: string; body: string; side?: 'top' | 'bottom' | 'left' | 'right' };

const TOURS: Record<TourName, { label: string; steps: Step[] }> = {
	welcome: {
		label: 'Welcome tour',
		steps: [
			{
				title: 'Welcome to Tuffie',
				body: 'Tuffie is a fast UI for Terraform and OpenTofu. It works next to the CLI: plan review, state browsing, approvals, policies and drift detection, in one small binary.<br><br>This tour takes about one minute. Use <kbd>→</kbd> and <kbd>←</kbd>, or <kbd>Esc</kbd> to stop.'
			},
			{ el: 'nav-workspaces', title: 'Workspaces', body: 'A workspace is one Terraform configuration: a folder on this machine or a Git repository. Each has its own runs, state, variables and settings.', side: 'right' },
			{ el: 'workspace-table', title: 'All workspaces at a glance', body: 'See the latest run, pending changes, resource count and drift for every workspace. A yellow badge means a plan waits for you or drift was found.', side: 'top' },
			{ el: 'new-workspace', title: 'Add your configuration', body: 'Point Tuffie at a folder with <code>.tf</code> files, or at a Git repository. Choose Terraform or OpenTofu per workspace.', side: 'bottom' },
			{ el: 'sample-project', title: 'No project at hand?', body: 'Create the sample project. It uses only the <code>random</code> provider, so it needs no cloud account.', side: 'top' },
			{ el: 'nav-varsets', title: 'Variable sets', body: 'Share variables such as cloud credentials or tags with all workspaces, or with the ones you choose. Every value is encrypted.', side: 'right' },
			{ el: 'nav-policies', title: 'Policies', body: 'Check every plan with Rego (OPA) rules. A mandatory policy that fails blocks the apply.', side: 'right' },
			{ el: 'mode', title: 'Local mode', body: 'Tuffie listens only on this machine and needs no login. Your runs see the same credentials as your shell.', side: 'right' },
			{ el: 'help', title: 'Help is always here', body: 'Open the tours again, read the documentation or see keyboard shortcuts. Press <kbd>?</kbd> anywhere.', side: 'right' },
			{
				title: 'You are ready',
				body: 'Open a workspace to continue. A short workspace tour starts the first time you open one.'
			}
		]
	},
	workspace: {
		label: 'Workspace tour',
		steps: [
			{ el: 'ws-header', title: 'The workspace', body: 'The name, the folder where Terraform runs, and the tool version. A <b>Locked</b> badge appears while a run holds the workspace.', side: 'bottom' },
			{ el: 'start-run', title: 'Start a run', body: '<b>Plan and apply</b> is the normal flow: Tuffie plans, you review, then you confirm. Also here: plan only, check for drift, refresh state and destroy.', side: 'bottom' },
			{ el: 'terminal-btn', title: 'Integrated terminal', body: 'Run <code>terraform</code>, <code>tofu</code> or <code>tf</code> commands in this folder, with all variables set. Press <kbd>Ctrl</kbd>+<kbd>`</kbd>. Commands are saved in the run history.', side: 'bottom' },
			{ el: 'ws-tabs', title: 'Everything about this workspace', body: '<b>Runs</b>: history. <b>State</b>: resources, graph and history. <b>Variables</b>: values and inherited sets. <b>Files</b>: edit <code>.tf</code> files. <b>Settings</b>: tool, auto-apply and drift schedule.', side: 'bottom' },
			{ el: 'latest-run', title: 'Latest run', body: 'The status of the last plan or apply. When a plan waits for confirmation, a <b>Review</b> button appears here.', side: 'bottom' },
			{ el: 'drift-card', title: 'Drift', body: 'Did someone change infrastructure outside Terraform? Click <b>Check now</b>, or set a schedule in Settings.', side: 'left' },
			{ el: 'resources-card', title: 'Resources', body: 'A summary of what this workspace manages. Open the State tab for every attribute and the dependency graph.', side: 'top' }
		]
	},
	run: {
		label: 'Run review tour',
		steps: [
			{ el: 'run-summary', title: 'The run', body: 'Status, kind, duration, and whether this run holds the workspace lock.', side: 'bottom' },
			{ el: 'plan-step', title: 'Plan', body: 'The result of <code>init</code> and <code>plan</code>. The counts show resources to add, change and destroy.', side: 'bottom' },
			{ el: 'plan-filter', title: 'Read the changes', body: 'Filter by action, or search by address. Click a resource to see every attribute: old value, new value, and values known only after apply.', side: 'bottom' },
			{ el: 'policy-step', title: 'Policy check', body: 'The policy sets that apply to this workspace. A failed mandatory set blocks the apply.', side: 'top' },
			{ el: 'apply-step', title: 'Confirm and apply', body: 'Tuffie applies exactly the plan you read. Add a comment, then confirm. Or discard the run to release the lock.', side: 'top' },
			{ el: 'run-variables', title: 'Where values came from', body: 'Every variable the run used, with its source: the workspace or a variable set. Values are never shown here.', side: 'top' }
		]
	}
};

const KEY = (t: TourName) => `tuffie-tour:${t}`;

export function tourDone(t: TourName): boolean {
	try {
		return localStorage.getItem(KEY(t)) === 'done';
	} catch {
		return true; // No storage: never auto-start.
	}
}

function markDone(t: TourName) {
	try {
		localStorage.setItem(KEY(t), 'done');
	} catch {
		/* storage unavailable */
	}
}

export function resetTours() {
	for (const t of Object.keys(TOURS) as TourName[]) {
		try {
			localStorage.removeItem(KEY(t));
		} catch {
			/* storage unavailable */
		}
	}
}

export const tourList = (Object.keys(TOURS) as TourName[]).map((name) => ({ name, label: TOURS[name].label }));

let active: Driver | null = null;

/** Start a tour. Without `force`, it starts only if the person has not seen it. */
export function startTour(name: TourName, force = false) {
	if (!force && tourDone(name)) return;
	if (active) active.destroy();
	const steps: DriveStep[] = TOURS[name].steps
		.filter((s) => !s.el || document.querySelector(`[data-tour="${s.el}"]`))
		.map((s) => ({
			element: s.el ? `[data-tour="${s.el}"]` : undefined,
			popover: { title: s.title, description: s.body, side: s.side ?? 'bottom', align: 'start' }
		}));
	if (!steps.length) return;

	// The last welcome step offers the sample project on an empty install.
	const last = steps[steps.length - 1];
	if (name === 'welcome' && last.popover) {
		last.popover.onPopoverRender = (popover) => {
			if (document.querySelector('[data-tour="sample-project"]')) {
				const btn = document.createElement('button');
				btn.textContent = 'Create sample project';
				btn.className = 'tuffie-tour-cta';
				btn.onclick = async () => {
					btn.disabled = true;
					try {
						const ws = await api.createSampleWorkspace();
						active?.destroy();
						goto(`/workspaces/${ws.id}`);
					} catch {
						btn.disabled = false;
					}
				};
				popover.footerButtons.prepend(btn);
			}
		};
	}

	active = driver({
		steps,
		showProgress: true,
		progressText: '{{current}} of {{total}}',
		nextBtnText: 'Next',
		prevBtnText: 'Back',
		doneBtnText: 'Done',
		popoverClass: 'tuffie-tour',
		overlayColor: '#000',
		overlayOpacity: 0.55,
		stagePadding: 6,
		stageRadius: 8,
		smoothScroll: true,
		allowClose: true,
		onDestroyed: () => {
			markDone(name);
			active = null;
		}
	});
	active.drive();
}

/** Start a tour after the page has rendered, if the person has not seen it. */
export function autoTour(name: TourName, delay = 700) {
	if (tourDone(name)) return;
	setTimeout(() => {
		// Do not stack tours, and do not start over an open dialog.
		if (active || document.querySelector('[role="dialog"]')) return;
		startTour(name);
	}, delay);
}

export function tourForPath(path: string): TourName {
	if (/^\/workspaces\/[^/]+\/runs\/[^/]+/.test(path)) return 'run';
	if (/^\/workspaces\/(?!new)[^/]+/.test(path)) return 'workspace';
	return 'welcome';
}
