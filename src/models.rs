use chrono::{DateTime, Utc};
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Short, readable ids in the style `ws-3fK9a0bQ2LmX7pZc`.
pub fn new_id(prefix: &str) -> String {
    let s: String = rand::thread_rng().sample_iter(&Alphanumeric).take(16).map(char::from).collect();
    format!("{prefix}-{s}")
}

pub fn now() -> DateTime<Utc> {
    Utc::now()
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source_kind: String,
    pub local_path: Option<String>,
    pub git_url: Option<String>,
    pub git_ref: Option<String>,
    pub working_dir: String,
    pub tool: String,
    pub tool_path: Option<String>,
    pub auto_apply: bool,
    pub lock_run_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunKind {
    PlanOnly,
    PlanAndApply,
    Destroy,
    Refresh,
    Cli,
}

impl RunKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunKind::PlanOnly => "plan_only",
            RunKind::PlanAndApply => "plan_and_apply",
            RunKind::Destroy => "destroy",
            RunKind::Refresh => "refresh",
            RunKind::Cli => "cli",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "plan_only" => RunKind::PlanOnly,
            "plan_and_apply" => RunKind::PlanAndApply,
            "destroy" => RunKind::Destroy,
            "refresh" => RunKind::Refresh,
            "cli" => RunKind::Cli,
            _ => return None,
        })
    }
}

/// Run status values. Stored as text.
pub mod status {
    pub const PENDING: &str = "pending";
    pub const PLANNING: &str = "planning";
    /// Plan has changes and waits for a person to confirm.
    pub const PLANNED: &str = "planned";
    /// Plan finished and nothing more happens (no changes, or plan-only run).
    pub const PLANNED_AND_FINISHED: &str = "planned_and_finished";
    pub const APPLYING: &str = "applying";
    pub const APPLIED: &str = "applied";
    pub const DISCARDED: &str = "discarded";
    pub const CANCELED: &str = "canceled";
    pub const ERRORED: &str = "errored";
    /// Terminal (CLI) commands.
    pub const RUNNING: &str = "running";
    pub const SUCCEEDED: &str = "succeeded";
    pub const FAILED: &str = "failed";

    pub fn is_final(s: &str) -> bool {
        matches!(s, PLANNED_AND_FINISHED | APPLIED | DISCARDED | CANCELED | ERRORED | SUCCEEDED | FAILED)
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Run {
    pub id: String,
    pub workspace_id: String,
    pub kind: String,
    pub source: String,
    pub status: String,
    pub message: String,
    pub command: Option<String>,
    pub has_changes: Option<bool>,
    pub add_count: i64,
    pub change_count: i64,
    pub destroy_count: i64,
    pub import_count: i64,
    pub exit_code: Option<i64>,
    pub has_state_snapshot: bool,
    pub state_serial: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub plan_finished_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub confirmed_comment: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct VariableRow {
    pub id: String,
    pub workspace_id: String,
    pub key: String,
    pub value_enc: String,
    pub category: String,
    pub sensitive: bool,
    pub hcl: bool,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Variable as the API returns it. `value` is `None` for sensitive variables:
/// sensitive values are write-only.
#[derive(Debug, Clone, Serialize)]
pub struct Variable {
    pub id: String,
    pub workspace_id: String,
    pub key: String,
    pub value: Option<String>,
    pub category: String,
    pub sensitive: bool,
    pub hcl: bool,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// One line in a run's event log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEvent {
    pub seq: u64,
    pub ts: DateTime<Utc>,
    /// `init`, `plan`, `apply`, `system`, or `cli`.
    pub stage: String,
    /// `info`, `warn` or `error`.
    pub level: String,
    pub message: String,
    /// Machine-readable message type from `-json` output (`planned_change`,
    /// `change_summary`, `diagnostic`, `apply_complete`, ...), or `status`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}
