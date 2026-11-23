export type Tool = 'terraform' | 'tofu';

export interface Workspace {
	id: string;
	name: string;
	description: string;
	source_kind: 'local' | 'git';
	local_path: string | null;
	git_url: string | null;
	git_ref: string | null;
	working_dir: string;
	tool: Tool;
	tool_path: string | null;
	auto_apply: boolean;
	lock_run_id: string | null;
	created_at: string;
	updated_at: string;
	drift_interval_minutes: number;
	drift_status: 'ok' | 'drifted' | 'error' | null;
	drift_count: number;
	drift_checked_at: string | null;
	drift_run_id: string | null;
}

export interface WorkspaceSummary extends Workspace {
	latest_run: Run | null;
	resource_count: number | null;
}

export interface WorkspaceDetail extends Workspace {
	resolved_dir: string;
	dir_exists: boolean;
	initialized: boolean;
	commit: { sha: string; subject: string; author: string; date: string } | null;
	tool_resolved_path: string | null;
	tool_version: string | null;
}

export type RunKind = 'plan_only' | 'plan_and_apply' | 'destroy' | 'refresh' | 'drift' | 'cli';

export type RunStatus =
	| 'pending'
	| 'planning'
	| 'planned'
	| 'planned_and_finished'
	| 'applying'
	| 'applied'
	| 'discarded'
	| 'canceled'
	| 'errored'
	| 'policy_failed'
	| 'running'
	| 'succeeded'
	| 'failed';

export interface Run {
	id: string;
	workspace_id: string;
	kind: RunKind;
	source: 'ui' | 'terminal' | 'api' | 'schedule';
	status: RunStatus;
	message: string;
	command: string | null;
	has_changes: boolean | null;
	add_count: number;
	change_count: number;
	destroy_count: number;
	import_count: number;
	drift_count: number;
	policy_status: 'passed' | 'warned' | 'failed' | 'errored' | null;
	exit_code: number | null;
	has_state_snapshot: boolean;
	state_serial: number | null;
	created_at: string;
	started_at: string | null;
	plan_finished_at: string | null;
	finished_at: string | null;
	confirmed_comment: string | null;
}

export interface RunDetail extends Run {
	workspace_name: string;
	queue_position: number | null;
	holds_lock: boolean;
}

export interface RunEvent {
	seq: number;
	ts: string;
	stage: string;
	level: 'info' | 'warn' | 'error' | 'debug' | 'trace';
	message: string;
	type?: string;
	data?: any;
}

export type Action = 'create' | 'update' | 'delete' | 'read' | 'no-op';

export interface ResourceChange {
	address: string;
	module_address: string | null;
	mode: 'managed' | 'data';
	type: string;
	name: string;
	provider_name: string;
	action_reason: string | null;
	actions: Action[];
	before: any;
	after: any;
	after_unknown: any;
	replace_paths: any;
	importing: any;
}

export interface Plan {
	terraform_version: string;
	resource_changes: ResourceChange[];
	output_changes: { name: string; actions: Action[]; before: any; after: any; after_unknown: any }[];
	resource_drift: number;
	drift: { address: string; type: string; mode: string; provider_name: string; actions: Action[]; before: any; after: any }[];
}

export interface Variable {
	id: string;
	workspace_id: string | null;
	set_id: string | null;
	key: string;
	value: string | null;
	category: 'terraform' | 'env';
	sensitive: boolean;
	hcl: boolean;
	description: string;
	created_at: string;
	updated_at: string;
}

export interface StateResource {
	address: string;
	module: string | null;
	mode: 'managed' | 'data';
	type: string;
	name: string;
	index?: any;
	provider_name: string;
	values: Record<string, any>;
	depends_on: string[] | null;
	tainted: boolean;
}

export interface StateView {
	initialized?: boolean;
	terraform_version?: string;
	serial?: number;
	resources: StateResource[];
	outputs: { name: string; sensitive: boolean; type: any; value: any }[];
	graph: { nodes: { id: string; type: string; mode: string }[]; edges: { from: string; to: string }[] };
}

export interface StateVersion {
	run_id: string;
	serial: number | null;
	created_at: string;
	kind: RunKind;
}

export interface FileEntry {
	path: string;
	name: string;
	dir: boolean;
	size: number;
	children: FileEntry[] | null;
}

export interface SystemInfo {
	version: string;
	mode: 'local' | 'server';
	auth: boolean;
	data_dir: string;
	inherit_env: boolean;
	tools: { name: Tool; path: string | null; version: string | null }[];
}

export interface VariableSet {
	id: string;
	name: string;
	description: string;
	global: boolean;
	workspace_ids: string[];
	variables: Variable[];
	created_at: string;
	updated_at: string;
}

export interface Policy {
	id: string;
	set_id: string;
	name: string;
	rego: string;
	created_at: string;
	updated_at: string;
}

export interface PolicySet {
	id: string;
	name: string;
	description: string;
	enforcement: 'advisory' | 'mandatory';
	global: boolean;
	source_kind: 'inline' | 'path';
	source_path: string | null;
	workspace_ids: string[];
	policies: Policy[];
	files: string[] | null;
	load_error: string | null;
	created_at: string;
	updated_at: string;
}

export interface PolicyFinding {
	level: 'fail' | 'warn';
	package: string;
	rule: string;
	message: string;
	details?: Record<string, any>;
}

export interface PolicySetResult {
	set_id: string;
	set_name: string;
	enforcement: 'advisory' | 'mandatory';
	status: 'passed' | 'warned' | 'failed' | 'errored';
	policy_count: number;
	error: string | null;
	findings: PolicyFinding[];
}

export interface PolicyReport {
	status: 'passed' | 'warned' | 'failed' | 'errored';
	sets: PolicySetResult[];
}

export interface VarOrigin {
	key: string;
	category: 'terraform' | 'env';
	source: 'workspace' | 'variable_set';
	set_id: string | null;
	set_name: string | null;
	sensitive: boolean;
	hcl: boolean;
	overridden: boolean;
	overridden_by: string | null;
}
