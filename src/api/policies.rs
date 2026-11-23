//! Policy sets and policies.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::assign::{self, Kind, Scope};
use crate::error::{bad, ApiError, ApiResult};
use crate::models::{new_id, now, Policy, PolicySet};
use crate::{policy, AppState};

#[derive(Serialize)]
pub struct SetView {
    #[serde(flatten)]
    set: PolicySet,
    workspace_ids: Vec<String>,
    policies: Vec<Policy>,
    /// For `path` sets: the files found, or the error.
    files: Option<Vec<String>>,
    load_error: Option<String>,
}

async fn view(state: &AppState, id: &str) -> ApiResult<SetView> {
    let set: PolicySet = sqlx::query_as("SELECT * FROM policy_sets WHERE id = ?").bind(id).fetch_one(&state.db).await?;
    let policies: Vec<Policy> = sqlx::query_as("SELECT * FROM policies WHERE set_id = ? ORDER BY name").bind(id).fetch_all(&state.db).await?;
    let (files, load_error) = if set.source_kind == "path" {
        match policy::load_sources(state, &set).await {
            Ok(s) => (Some(s.into_iter().map(|(n, _)| n).collect()), None),
            Err(e) => (None, Some(format!("{e:#}"))),
        }
    } else {
        (None, None)
    };
    Ok(SetView { workspace_ids: assign::workspace_ids(state, Kind::Policy, id).await?, policies, files, load_error, set })
}

pub async fn list(State(state): State<AppState>) -> ApiResult<Json<Vec<SetView>>> {
    let ids: Vec<(String,)> = sqlx::query_as("SELECT id FROM policy_sets ORDER BY name").fetch_all(&state.db).await?;
    let mut out = vec![];
    for (id,) in ids {
        out.push(view(&state, &id).await?);
    }
    Ok(Json(out))
}

pub async fn get(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<SetView>> {
    Ok(Json(view(&state, &id).await?))
}

#[derive(Deserialize)]
pub struct Upsert {
    name: Option<String>,
    description: Option<String>,
    enforcement: Option<String>,
    source_kind: Option<String>,
    source_path: Option<String>,
    #[serde(flatten)]
    scope: Scope,
}

fn check_enforcement(e: &str) -> ApiResult<()> {
    if !matches!(e, "advisory" | "mandatory") {
        return Err(bad("enforcement must be `advisory` or `mandatory`"));
    }
    Ok(())
}

fn check_source(kind: &str, path: Option<&str>) -> ApiResult<()> {
    match kind {
        "inline" => Ok(()),
        "path" => {
            let p = path.filter(|p| !p.is_empty()).ok_or_else(|| bad("a folder path is required"))?;
            let p = std::path::Path::new(p);
            if !p.is_absolute() || !p.is_dir() {
                return Err(bad("the policy folder must be an absolute path to an existing folder"));
            }
            Ok(())
        }
        _ => Err(bad("source_kind must be `inline` or `path`")),
    }
}

pub async fn create(State(state): State<AppState>, Json(req): Json<Upsert>) -> ApiResult<(StatusCode, Json<SetView>)> {
    let name = req.name.unwrap_or_default();
    super::varsets::validate_name(&name)?;
    let enforcement = req.enforcement.unwrap_or_else(|| "advisory".into());
    check_enforcement(&enforcement)?;
    let source_kind = req.source_kind.unwrap_or_else(|| "inline".into());
    check_source(&source_kind, req.source_path.as_deref())?;
    let id = new_id("polset");
    let t = now();
    sqlx::query("INSERT INTO policy_sets (id, name, description, enforcement, global, source_kind, source_path, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(name.trim())
        .bind(req.description.unwrap_or_default())
        .bind(&enforcement)
        .bind(req.scope.global.unwrap_or(false))
        .bind(&source_kind)
        .bind(req.source_path.filter(|_| source_kind == "path"))
        .bind(t)
        .bind(t)
        .execute(&state.db)
        .await?;
    assign::set_workspaces(&state, Kind::Policy, &id, &req.scope.workspace_ids.unwrap_or_default()).await?;
    Ok((StatusCode::CREATED, Json(view(&state, &id).await?)))
}

pub async fn update(State(state): State<AppState>, Path(id): Path<String>, Json(req): Json<Upsert>) -> ApiResult<Json<SetView>> {
    let mut set: PolicySet = sqlx::query_as("SELECT * FROM policy_sets WHERE id = ?").bind(&id).fetch_one(&state.db).await?;
    if let Some(n) = req.name {
        super::varsets::validate_name(&n)?;
        set.name = n.trim().to_string();
    }
    if let Some(d) = req.description {
        set.description = d;
    }
    if let Some(e) = req.enforcement {
        check_enforcement(&e)?;
        set.enforcement = e;
    }
    if let Some(g) = req.scope.global {
        set.global = g;
    }
    if req.source_kind.is_some() || req.source_path.is_some() {
        let kind = req.source_kind.unwrap_or(set.source_kind.clone());
        let path = req.source_path.or(set.source_path.clone());
        check_source(&kind, path.as_deref())?;
        set.source_path = path.filter(|_| kind == "path");
        set.source_kind = kind;
    }
    sqlx::query("UPDATE policy_sets SET name = ?, description = ?, enforcement = ?, global = ?, source_kind = ?, source_path = ?, updated_at = ? WHERE id = ?")
        .bind(&set.name)
        .bind(&set.description)
        .bind(&set.enforcement)
        .bind(set.global)
        .bind(&set.source_kind)
        .bind(&set.source_path)
        .bind(now())
        .bind(&id)
        .execute(&state.db)
        .await?;
    if let Some(ids) = req.scope.workspace_ids {
        assign::set_workspaces(&state, Kind::Policy, &id, &ids).await?;
    }
    Ok(Json(view(&state, &id).await?))
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<StatusCode> {
    sqlx::query("DELETE FROM policy_sets WHERE id = ?").bind(&id).execute(&state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct PolicyBody {
    name: Option<String>,
    rego: Option<String>,
}

fn check_policy_name(n: &str) -> ApiResult<()> {
    let ok = !n.is_empty() && n.len() <= 120 && n.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if !ok {
        return Err(bad("policy name must use letters, numbers, `-`, `_` and `.`"));
    }
    Ok(())
}

/// Reject Rego that does not compile, with the parser's message.
fn check_rego(name: &str, rego: &str) -> ApiResult<()> {
    if rego.len() > 256 * 1024 {
        return Err(bad("policy is larger than 256 KB"));
    }
    policy::compile(&[(name.to_string(), rego.to_string())]).map(|_| ()).map_err(|e| bad(format!("{e}")))
}

pub async fn add_policy(State(state): State<AppState>, Path(set_id): Path<String>, Json(req): Json<PolicyBody>) -> ApiResult<(StatusCode, Json<Policy>)> {
    let set: PolicySet = sqlx::query_as("SELECT * FROM policy_sets WHERE id = ?").bind(&set_id).fetch_one(&state.db).await?;
    if set.source_kind != "inline" {
        return Err(bad("this policy set reads its policies from a folder"));
    }
    let mut name = req.name.unwrap_or_default();
    if !name.ends_with(".rego") {
        name.push_str(".rego");
    }
    check_policy_name(&name)?;
    let rego = req.rego.unwrap_or_default();
    check_rego(&name, &rego)?;
    let id = new_id("pol");
    let t = now();
    sqlx::query("INSERT INTO policies (id, set_id, name, rego, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(&set_id)
        .bind(&name)
        .bind(&rego)
        .bind(t)
        .bind(t)
        .execute(&state.db)
        .await?;
    Ok((StatusCode::CREATED, Json(sqlx::query_as("SELECT * FROM policies WHERE id = ?").bind(&id).fetch_one(&state.db).await?)))
}

pub async fn update_policy(State(state): State<AppState>, Path(id): Path<String>, Json(req): Json<PolicyBody>) -> ApiResult<Json<Policy>> {
    let mut p: Policy = sqlx::query_as("SELECT * FROM policies WHERE id = ?").bind(&id).fetch_one(&state.db).await?;
    if let Some(n) = req.name {
        check_policy_name(&n)?;
        p.name = n;
    }
    if let Some(r) = req.rego {
        check_rego(&p.name, &r)?;
        p.rego = r;
    }
    sqlx::query("UPDATE policies SET name = ?, rego = ?, updated_at = ? WHERE id = ?")
        .bind(&p.name)
        .bind(&p.rego)
        .bind(now())
        .bind(&id)
        .execute(&state.db)
        .await?;
    Ok(Json(sqlx::query_as("SELECT * FROM policies WHERE id = ?").bind(&id).fetch_one(&state.db).await?))
}

pub async fn delete_policy(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<StatusCode> {
    sqlx::query("DELETE FROM policies WHERE id = ?").bind(&id).execute(&state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct TestReq {
    run_id: String,
}

/// Evaluate a policy set against the saved plan of an earlier run.
pub async fn test(State(state): State<AppState>, Path(id): Path<String>, Json(req): Json<TestReq>) -> ApiResult<Json<Value>> {
    let set: PolicySet = sqlx::query_as("SELECT * FROM policy_sets WHERE id = ?").bind(&id).fetch_one(&state.db).await?;
    crate::runner::get_run(&state, &req.run_id).await?;
    let path = crate::runner::run_dir(&state, &req.run_id).join("plan.json.enc");
    if !path.exists() {
        return Err(ApiError::NotFound("this run has no saved plan".into()));
    }
    let plan = state.crypto.read_encrypted(&path, &format!("{}/plan", req.run_id))?;
    let result = policy::run_set(&state, &set, &String::from_utf8_lossy(&plan)).await;
    Ok(Json(json!(result)))
}

/// Recent runs that have a saved plan, to pick from in the test dialog.
pub async fn testable_runs(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let rows: Vec<(String, String, String, String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT r.id, w.name, r.kind, r.message, r.created_at FROM runs r JOIN workspaces w ON w.id = r.workspace_id
         WHERE r.plan_finished_at IS NOT NULL AND r.kind NOT IN ('cli', 'drift') ORDER BY r.created_at DESC LIMIT 50",
    )
    .fetch_all(&state.db)
    .await?;
    let mut out = vec![];
    for (id, ws, kind, message, at) in rows {
        if crate::runner::run_dir(&state, &id).join("plan.json.enc").exists() {
            out.push(json!({ "run_id": id, "workspace": ws, "kind": kind, "message": message, "created_at": at }));
        }
    }
    Ok(Json(json!(out)))
}
