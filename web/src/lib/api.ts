import type {
	FileEntry,
	Plan,
	Policy,
	PolicyReport,
	PolicySet,
	PolicySetResult,
	VarOrigin,
	VariableSet,
	Run,
	RunDetail,
	RunEvent,
	RunKind,
	StateVersion,
	StateView,
	SystemInfo,
	Variable,
	Workspace,
	WorkspaceDetail,
	WorkspaceSummary
} from './types';

export class ApiError extends Error {
	constructor(
		public status: number,
		public code: string,
		message: string
	) {
		super(message);
	}
}

async function req<T>(method: string, path: string, body?: unknown): Promise<T> {
	const res = await fetch(`/api${path}`, {
		method,
		headers: body !== undefined ? { 'content-type': 'application/json' } : undefined,
		body: body !== undefined ? JSON.stringify(body) : undefined
	});
	if (!res.ok) {
		let code = 'error';
		let message = res.statusText;
		try {
			const j = await res.json();
			code = j.error?.code ?? code;
			message = j.error?.message ?? message;
		} catch {
			/* not JSON */
		}
		throw new ApiError(res.status, code, message);
	}
	if (res.status === 204 || res.status === 202) return undefined as T;
	const ct = res.headers.get('content-type') ?? '';
	return (ct.includes('json') ? res.json() : res.text()) as Promise<T>;
}

const enc = encodeURIComponent;

export const api = {
	system: () => req<SystemInfo>('GET', '/system/info'),

	workspaces: () => req<WorkspaceSummary[]>('GET', '/workspaces'),
	workspace: (id: string) => req<WorkspaceDetail>('GET', `/workspaces/${id}`),
	createWorkspace: (b: Partial<Workspace>) => req<Workspace>('POST', '/workspaces', b),
	updateWorkspace: (id: string, b: Partial<Workspace>) => req<Workspace>('PATCH', `/workspaces/${id}`, b),
	deleteWorkspace: (id: string) => req<void>('DELETE', `/workspaces/${id}`),
	syncWorkspace: (id: string) => req<{ sha: string }>('POST', `/workspaces/${id}/sync`),
	createSampleWorkspace: () => req<Workspace>('POST', '/workspaces/sample'),
	checkDrift: (id: string) => req<Run>('POST', `/workspaces/${id}/drift`),
	workspaceVariableSets: (id: string) => req<VariableSet[]>('GET', `/workspaces/${id}/variable-sets`),

	variableSets: () => req<VariableSet[]>('GET', '/variable-sets'),
	variableSet: (id: string) => req<VariableSet>('GET', `/variable-sets/${id}`),
	createVariableSet: (b: Partial<VariableSet>) => req<VariableSet>('POST', '/variable-sets', b),
	updateVariableSet: (id: string, b: Partial<VariableSet>) => req<VariableSet>('PATCH', `/variable-sets/${id}`, b),
	deleteVariableSet: (id: string) => req<void>('DELETE', `/variable-sets/${id}`),
	createSetVariable: (set: string, b: Partial<Variable>) => req<Variable>('POST', `/variable-sets/${set}/variables`, b),

	policySets: () => req<PolicySet[]>('GET', '/policy-sets'),
	policySet: (id: string) => req<PolicySet>('GET', `/policy-sets/${id}`),
	createPolicySet: (b: Partial<PolicySet>) => req<PolicySet>('POST', '/policy-sets', b),
	updatePolicySet: (id: string, b: Partial<PolicySet>) => req<PolicySet>('PATCH', `/policy-sets/${id}`, b),
	deletePolicySet: (id: string) => req<void>('DELETE', `/policy-sets/${id}`),
	createPolicy: (set: string, b: Partial<Policy>) => req<Policy>('POST', `/policy-sets/${set}/policies`, b),
	updatePolicy: (id: string, b: Partial<Policy>) => req<Policy>('PATCH', `/policies/${id}`, b),
	deletePolicy: (id: string) => req<void>('DELETE', `/policies/${id}`),
	testPolicySet: (id: string, runId: string) => req<PolicySetResult>('POST', `/policy-sets/${id}/test`, { run_id: runId }),
	testableRuns: () =>
		req<{ run_id: string; workspace: string; kind: RunKind; message: string; created_at: string }[]>('GET', '/policy-sets/testable-runs'),

	runs: (ws: string, limit = 50) => req<Run[]>('GET', `/workspaces/${ws}/runs?limit=${limit}`),
	createRun: (ws: string, kind: RunKind, message = '') => req<Run>('POST', `/workspaces/${ws}/runs`, { kind, message }),
	run: (id: string) => req<RunDetail>('GET', `/runs/${id}`),
	runEvents: (id: string) => req<RunEvent[]>('GET', `/runs/${id}/events`),
	runPlan: (id: string) => req<Plan>('GET', `/runs/${id}/plan`),
	runOutput: (id: string) => req<string>('GET', `/runs/${id}/output`),
	runState: (id: string) => req<StateView>('GET', `/runs/${id}/state`),
	runPolicy: (id: string) => req<PolicyReport>('GET', `/runs/${id}/policy`),
	runVariables: (id: string) => req<VarOrigin[]>('GET', `/runs/${id}/variables`),
	applyRun: (id: string, comment?: string) => req<void>('POST', `/runs/${id}/apply`, { comment }),
	discardRun: (id: string) => req<void>('POST', `/runs/${id}/discard`),
	cancelRun: (id: string) => req<void>('POST', `/runs/${id}/cancel`),

	variables: (ws: string) => req<Variable[]>('GET', `/workspaces/${ws}/variables`),
	createVariable: (ws: string, b: Partial<Variable>) => req<Variable>('POST', `/workspaces/${ws}/variables`, b),
	updateVariable: (id: string, b: Partial<Variable>) => req<Variable>('PATCH', `/variables/${id}`, b),
	deleteVariable: (id: string) => req<void>('DELETE', `/variables/${id}`),

	files: (ws: string) => req<FileEntry[]>('GET', `/workspaces/${ws}/files`),
	readFile: (ws: string, path: string) =>
		req<{ path: string; content: string; size: number }>('GET', `/workspaces/${ws}/files/content?path=${enc(path)}`),
	writeFile: (ws: string, path: string, content: string) =>
		req<void>('PUT', `/workspaces/${ws}/files/content?path=${enc(path)}`, { content }),

	state: (ws: string) => req<StateView>('GET', `/workspaces/${ws}/state`),
	stateVersions: (ws: string) => req<StateVersion[]>('GET', `/workspaces/${ws}/state-versions`)
};

export function wsUrl(path: string): string {
	const proto = location.protocol === 'https:' ? 'wss' : 'ws';
	return `${proto}://${location.host}/api${path}`;
}

export function errMsg(e: unknown): string {
	return e instanceof Error ? e.message : String(e);
}
