use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::error::{bad, ApiResult};
use crate::models::{new_id, now, Variable, VariableRow};
use crate::runner::get_workspace;
use crate::AppState;

pub fn to_api(state: &AppState, row: VariableRow) -> anyhow::Result<Variable> {
    // Sensitive values are write-only: the API never returns them.
    let value = if row.sensitive { None } else { Some(state.crypto.decrypt(&row.value_enc, &row.id)?.to_string()) };
    Ok(Variable {
        id: row.id,
        workspace_id: row.workspace_id,
        set_id: row.set_id,
        key: row.key,
        value,
        category: row.category,
        sensitive: row.sensitive,
        hcl: row.hcl,
        description: row.description,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn validate(key: &str, category: &str) -> ApiResult<()> {
    if !matches!(category, "terraform" | "env") {
        return Err(bad("category must be `terraform` or `env`"));
    }
    let ok = !key.is_empty()
        && key.len() <= 128
        && key.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || (category == "terraform" && c == '-'));
    if !ok {
        return Err(bad("key must start with a letter or `_` and use only letters, numbers and `_`"));
    }
    if category == "env" && (key.starts_with("TUFFIE_") || key == "PATH") {
        return Err(bad("this environment variable name is reserved"));
    }
    Ok(())
}

pub async fn list_for(state: &AppState, column: &str, owner: &str) -> ApiResult<Vec<Variable>> {
    // `column` is a fixed internal name, never user input.
    debug_assert!(column == "workspace_id" || column == "set_id");
    let rows: Vec<VariableRow> = sqlx::query_as(&format!("SELECT * FROM variables WHERE {column} = ? ORDER BY category, key"))
        .bind(owner)
        .fetch_all(&state.db)
        .await?;
    Ok(rows.into_iter().map(|r| to_api(state, r)).collect::<anyhow::Result<Vec<_>>>()?)
}

pub async fn list(State(state): State<AppState>, Path(ws): Path<String>) -> ApiResult<Json<Vec<Variable>>> {
    Ok(Json(list_for(&state, "workspace_id", &ws).await?))
}

#[derive(Deserialize)]
pub struct CreateVar {
    key: String,
    value: String,
    category: String,
    #[serde(default)]
    sensitive: bool,
    #[serde(default)]
    hcl: bool,
    #[serde(default)]
    description: String,
}

pub enum Owner<'a> {
    Workspace(&'a str),
    Set(&'a str),
}

pub async fn insert(state: &AppState, owner: Owner<'_>, req: CreateVar) -> ApiResult<Variable> {
    validate(&req.key, &req.category)?;
    let id = new_id("var");
    let enc = state.crypto.encrypt(&req.value, &id)?;
    let (ws, set) = match owner {
        Owner::Workspace(w) => (Some(w), None),
        Owner::Set(s) => (None, Some(s)),
    };
    let t = now();
    sqlx::query("INSERT INTO variables (id, workspace_id, set_id, key, value_enc, category, sensitive, hcl, description, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(ws)
        .bind(set)
        .bind(&req.key)
        .bind(&enc)
        .bind(&req.category)
        .bind(req.sensitive)
        .bind(req.hcl)
        .bind(&req.description)
        .bind(t)
        .bind(t)
        .execute(&state.db)
        .await?;
    let row: VariableRow = sqlx::query_as("SELECT * FROM variables WHERE id = ?").bind(&id).fetch_one(&state.db).await?;
    Ok(to_api(state, row)?)
}

pub async fn create(State(state): State<AppState>, Path(ws): Path<String>, Json(req): Json<CreateVar>) -> ApiResult<(StatusCode, Json<Variable>)> {
    get_workspace(&state, &ws).await?;
    Ok((StatusCode::CREATED, Json(insert(&state, Owner::Workspace(&ws), req).await?)))
}

#[derive(Deserialize)]
pub struct UpdateVar {
    key: Option<String>,
    /// Omit to keep the current value.
    value: Option<String>,
    sensitive: Option<bool>,
    hcl: Option<bool>,
    description: Option<String>,
}

pub async fn update(State(state): State<AppState>, Path(id): Path<String>, Json(req): Json<UpdateVar>) -> ApiResult<Json<Variable>> {
    let mut row: VariableRow = sqlx::query_as("SELECT * FROM variables WHERE id = ?").bind(&id).fetch_one(&state.db).await?;
    if let Some(k) = req.key {
        validate(&k, &row.category)?;
        row.key = k;
    }
    if let Some(v) = req.value {
        row.value_enc = state.crypto.encrypt(&v, &id)?;
    }
    if let Some(s) = req.sensitive {
        // A sensitive variable cannot become non-sensitive: that would reveal the value.
        if row.sensitive && !s {
            return Err(bad("a sensitive variable cannot be made non-sensitive; delete it and create it again"));
        }
        row.sensitive = s;
    }
    if let Some(h) = req.hcl {
        row.hcl = h;
    }
    if let Some(d) = req.description {
        row.description = d;
    }
    sqlx::query("UPDATE variables SET key = ?, value_enc = ?, sensitive = ?, hcl = ?, description = ?, updated_at = ? WHERE id = ?")
        .bind(&row.key)
        .bind(&row.value_enc)
        .bind(row.sensitive)
        .bind(row.hcl)
        .bind(&row.description)
        .bind(now())
        .bind(&id)
        .execute(&state.db)
        .await?;
    let row: VariableRow = sqlx::query_as("SELECT * FROM variables WHERE id = ?").bind(&id).fetch_one(&state.db).await?;
    Ok(Json(to_api(&state, row)?))
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<StatusCode> {
    sqlx::query("DELETE FROM variables WHERE id = ?").bind(&id).execute(&state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}
