//! Shared helpers for sets that are global or assigned to chosen workspaces
//! (variable sets and policy sets).

use serde::Deserialize;

use crate::error::{bad, ApiResult};
use crate::AppState;

#[derive(Clone, Copy)]
pub enum Kind {
    Variable,
    Policy,
}

impl Kind {
    fn link_table(self) -> &'static str {
        match self {
            Kind::Variable => "variable_set_workspaces",
            Kind::Policy => "policy_set_workspaces",
        }
    }
    fn set_table(self) -> &'static str {
        match self {
            Kind::Variable => "variable_sets",
            Kind::Policy => "policy_sets",
        }
    }
}

#[derive(Deserialize, Default)]
pub struct Scope {
    pub global: Option<bool>,
    pub workspace_ids: Option<Vec<String>>,
}

pub async fn workspace_ids(state: &AppState, kind: Kind, set_id: &str) -> ApiResult<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(&format!("SELECT workspace_id FROM {} WHERE set_id = ?", kind.link_table()))
        .bind(set_id)
        .fetch_all(&state.db)
        .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

/// Replace the workspace list of a set.
pub async fn set_workspaces(state: &AppState, kind: Kind, set_id: &str, ids: &[String]) -> ApiResult<()> {
    let mut tx = state.db.begin().await?;
    sqlx::query(&format!("DELETE FROM {} WHERE set_id = ?", kind.link_table())).bind(set_id).execute(&mut *tx).await?;
    for ws in ids {
        let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM workspaces WHERE id = ?").bind(ws).fetch_optional(&mut *tx).await?;
        if exists.is_none() {
            return Err(bad(format!("workspace {ws} does not exist")));
        }
        sqlx::query(&format!("INSERT INTO {} (set_id, workspace_id) VALUES (?, ?)", kind.link_table()))
            .bind(set_id)
            .bind(ws)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Ids of the sets that apply to a workspace: global sets first, then
/// assigned sets, each group sorted by name. Later sets win on conflicts.
pub async fn sets_for_workspace(state: &AppState, kind: Kind, ws_id: &str) -> ApiResult<Vec<String>> {
    let sql = format!(
        "SELECT id FROM {set} WHERE global = 1 OR id IN (SELECT set_id FROM {link} WHERE workspace_id = ?)
         ORDER BY global DESC, name ASC",
        set = kind.set_table(),
        link = kind.link_table()
    );
    let rows: Vec<(String,)> = sqlx::query_as(&sql).bind(ws_id).fetch_all(&state.db).await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}
