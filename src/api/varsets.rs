//! Variable sets: groups of variables shared by many workspaces.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::assign::{self, Kind, Scope};
use super::variables::{self, CreateVar, Owner};
use crate::error::{bad, ApiResult};
use crate::models::{new_id, now, Variable, VariableSet};
use crate::AppState;

#[derive(Serialize)]
pub struct SetView {
    #[serde(flatten)]
    set: VariableSet,
    workspace_ids: Vec<String>,
    variables: Vec<Variable>,
}

async fn view(state: &AppState, id: &str) -> ApiResult<SetView> {
    let set: VariableSet = sqlx::query_as("SELECT * FROM variable_sets WHERE id = ?").bind(id).fetch_one(&state.db).await?;
    Ok(SetView {
        workspace_ids: assign::workspace_ids(state, Kind::Variable, id).await?,
        variables: variables::list_for(state, "set_id", id).await?,
        set,
    })
}

pub async fn list(State(state): State<AppState>) -> ApiResult<Json<Vec<SetView>>> {
    let ids: Vec<(String,)> = sqlx::query_as("SELECT id FROM variable_sets ORDER BY name").fetch_all(&state.db).await?;
    let mut out = vec![];
    for (id,) in ids {
        out.push(view(&state, &id).await?);
    }
    Ok(Json(out))
}

pub async fn get(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<SetView>> {
    Ok(Json(view(&state, &id).await?))
}

pub fn validate_name(name: &str) -> ApiResult<()> {
    if name.trim().is_empty() || name.len() > 90 {
        return Err(bad("name must be 1-90 characters"));
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct Upsert {
    name: Option<String>,
    description: Option<String>,
    #[serde(flatten)]
    scope: Scope,
}

pub async fn create(State(state): State<AppState>, Json(req): Json<Upsert>) -> ApiResult<(StatusCode, Json<SetView>)> {
    let name = req.name.unwrap_or_default();
    validate_name(&name)?;
    let id = new_id("varset");
    let t = now();
    sqlx::query("INSERT INTO variable_sets (id, name, description, global, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(name.trim())
        .bind(req.description.unwrap_or_default())
        .bind(req.scope.global.unwrap_or(false))
        .bind(t)
        .bind(t)
        .execute(&state.db)
        .await?;
    assign::set_workspaces(&state, Kind::Variable, &id, &req.scope.workspace_ids.unwrap_or_default()).await?;
    Ok((StatusCode::CREATED, Json(view(&state, &id).await?)))
}

pub async fn update(State(state): State<AppState>, Path(id): Path<String>, Json(req): Json<Upsert>) -> ApiResult<Json<SetView>> {
    let mut set: VariableSet = sqlx::query_as("SELECT * FROM variable_sets WHERE id = ?").bind(&id).fetch_one(&state.db).await?;
    if let Some(n) = req.name {
        validate_name(&n)?;
        set.name = n.trim().to_string();
    }
    if let Some(d) = req.description {
        set.description = d;
    }
    if let Some(g) = req.scope.global {
        set.global = g;
    }
    sqlx::query("UPDATE variable_sets SET name = ?, description = ?, global = ?, updated_at = ? WHERE id = ?")
        .bind(&set.name)
        .bind(&set.description)
        .bind(set.global)
        .bind(now())
        .bind(&id)
        .execute(&state.db)
        .await?;
    if let Some(ids) = req.scope.workspace_ids {
        assign::set_workspaces(&state, Kind::Variable, &id, &ids).await?;
    }
    Ok(Json(view(&state, &id).await?))
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<StatusCode> {
    sqlx::query("DELETE FROM variable_sets WHERE id = ?").bind(&id).execute(&state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn add_variable(State(state): State<AppState>, Path(id): Path<String>, Json(req): Json<CreateVar>) -> ApiResult<(StatusCode, Json<Variable>)> {
    sqlx::query("SELECT id FROM variable_sets WHERE id = ?").bind(&id).fetch_one(&state.db).await?;
    Ok((StatusCode::CREATED, Json(variables::insert(&state, Owner::Set(&id), req).await?)))
}

/// Sets that apply to a workspace, with their variables, in precedence order.
pub async fn for_workspace(State(state): State<AppState>, Path(ws): Path<String>) -> ApiResult<Json<Value>> {
    let ids = assign::sets_for_workspace(&state, Kind::Variable, &ws).await?;
    let mut out = vec![];
    for id in ids {
        out.push(view(&state, &id).await?);
    }
    Ok(Json(json!(out)))
}
