use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{bad, ApiError, ApiResult};
use crate::models::{new_id, now, Run, Workspace};
use crate::runner::get_workspace;
use crate::{exec, git, AppState};

#[derive(Serialize)]
pub struct WorkspaceSummary {
    #[serde(flatten)]
    ws: Workspace,
    latest_run: Option<Run>,
    resource_count: Option<i64>,
}

pub async fn list(State(state): State<AppState>) -> ApiResult<Json<Vec<WorkspaceSummary>>> {
    let all: Vec<Workspace> = sqlx::query_as("SELECT * FROM workspaces ORDER BY name").fetch_all(&state.db).await?;
    let mut out = Vec::with_capacity(all.len());
    for ws in all {
        let latest_run: Option<Run> =
            sqlx::query_as("SELECT * FROM runs WHERE workspace_id = ? ORDER BY created_at DESC LIMIT 1")
                .bind(&ws.id)
                .fetch_optional(&state.db)
                .await?;
        let resource_count = super::state::cached_resource_count(&state, &ws.id).await;
        out.push(WorkspaceSummary { ws, latest_run, resource_count });
    }
    Ok(Json(out))
}

pub async fn get(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let ws = get_workspace(&state, &id).await?;
    let root = exec::source_root(&state, &ws)?;
    let dir = exec::workdir(&state, &ws)?;
    let commit = if ws.source_kind == "git" { git::head(&root).await } else { None };
    let commit = commit.map(|c| {
        let p: Vec<&str> = c.split('\0').collect();
        json!({ "sha": p.first(), "subject": p.get(1), "author": p.get(2), "date": p.get(3) })
    });
    let tool_path = exec::resolve_tool(&ws).ok();
    let tool_version = match &tool_path {
        Some(p) => exec::tool_version(p).await,
        None => None,
    };
    let mut v = serde_json::to_value(&ws).map_err(anyhow::Error::from)?;
    v["resolved_dir"] = json!(dir.display().to_string());
    v["dir_exists"] = json!(dir.is_dir());
    v["initialized"] = json!(dir.join(".terraform").is_dir());
    v["commit"] = json!(commit);
    v["tool_resolved_path"] = json!(tool_path.map(|p| p.display().to_string()));
    v["tool_version"] = json!(tool_version);
    Ok(Json(v))
}

#[derive(Deserialize)]
pub struct CreateWorkspace {
    name: String,
    #[serde(default)]
    description: String,
    source_kind: String,
    local_path: Option<String>,
    git_url: Option<String>,
    git_ref: Option<String>,
    #[serde(default)]
    working_dir: String,
    #[serde(default = "default_tool")]
    tool: String,
    tool_path: Option<String>,
    #[serde(default)]
    auto_apply: bool,
}

fn default_tool() -> String {
    "terraform".into()
}

fn validate_name(name: &str) -> ApiResult<()> {
    let ok = !name.is_empty()
        && name.len() <= 90
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !ok {
        return Err(bad("name must be 1-90 characters: letters, numbers, `-` and `_`"));
    }
    Ok(())
}

fn validate_tool(tool: &str) -> ApiResult<()> {
    if !exec::TOOLS.contains(&tool) {
        return Err(bad("tool must be `terraform` or `tofu`"));
    }
    Ok(())
}

pub async fn create(State(state): State<AppState>, Json(req): Json<CreateWorkspace>) -> ApiResult<(StatusCode, Json<Workspace>)> {
    validate_name(&req.name)?;
    validate_tool(&req.tool)?;
    exec::safe_relative(&req.working_dir).map_err(|e| bad(e.to_string()))?;
    let id = new_id("ws");
    match req.source_kind.as_str() {
        "local" => {
            let p = req.local_path.as_deref().filter(|p| !p.is_empty()).ok_or_else(|| bad("local_path is required"))?;
            let path = std::path::Path::new(p);
            if !path.is_absolute() {
                return Err(bad("local_path must be an absolute path"));
            }
            if !path.is_dir() {
                return Err(bad(format!("folder {p} does not exist")));
            }
        }
        "git" => {
            let url = req.git_url.as_deref().ok_or_else(|| bad("git_url is required"))?;
            git::validate_url(url).map_err(|e| bad(e.to_string()))?;
            if let Some(r) = req.git_ref.as_deref().filter(|r| !r.is_empty()) {
                git::validate_ref(r).map_err(|e| bad(e.to_string()))?;
            }
            let dest = state.config.checkouts_dir().join(&id);
            git::sync(&dest, url, req.git_ref.as_deref()).await.map_err(|e| bad(format!("{e:#}")))?;
        }
        _ => return Err(bad("source_kind must be `local` or `git`")),
    }
    let t = now();
    sqlx::query(
        "INSERT INTO workspaces (id, name, description, source_kind, local_path, git_url, git_ref, working_dir, tool, tool_path, auto_apply, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.source_kind)
    .bind(&req.local_path)
    .bind(&req.git_url)
    .bind(&req.git_ref)
    .bind(&req.working_dir)
    .bind(&req.tool)
    .bind(&req.tool_path)
    .bind(req.auto_apply)
    .bind(t)
    .bind(t)
    .execute(&state.db)
    .await?;
    Ok((StatusCode::CREATED, Json(get_workspace(&state, &id).await?)))
}

#[derive(Deserialize)]
pub struct UpdateWorkspace {
    name: Option<String>,
    description: Option<String>,
    git_ref: Option<String>,
    working_dir: Option<String>,
    tool: Option<String>,
    tool_path: Option<Option<String>>,
    auto_apply: Option<bool>,
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateWorkspace>,
) -> ApiResult<Json<Workspace>> {
    let mut ws = get_workspace(&state, &id).await?;
    if let Some(n) = req.name {
        validate_name(&n)?;
        ws.name = n;
    }
    if let Some(d) = req.description {
        ws.description = d;
    }
    if let Some(r) = req.git_ref {
        git::validate_ref(&r).map_err(|e| bad(e.to_string()))?;
        ws.git_ref = Some(r);
    }
    if let Some(w) = req.working_dir {
        exec::safe_relative(&w).map_err(|e| bad(e.to_string()))?;
        ws.working_dir = w;
    }
    if let Some(t) = req.tool {
        validate_tool(&t)?;
        ws.tool = t;
    }
    if let Some(p) = req.tool_path {
        ws.tool_path = p.filter(|s| !s.is_empty());
    }
    if let Some(a) = req.auto_apply {
        ws.auto_apply = a;
    }
    sqlx::query(
        "UPDATE workspaces SET name = ?, description = ?, git_ref = ?, working_dir = ?, tool = ?, tool_path = ?, auto_apply = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&ws.name)
    .bind(&ws.description)
    .bind(&ws.git_ref)
    .bind(&ws.working_dir)
    .bind(&ws.tool)
    .bind(&ws.tool_path)
    .bind(ws.auto_apply)
    .bind(now())
    .bind(&id)
    .execute(&state.db)
    .await?;
    Ok(Json(get_workspace(&state, &id).await?))
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<StatusCode> {
    let ws = get_workspace(&state, &id).await?;
    if ws.lock_run_id.is_some() {
        return Err(ApiError::Conflict("the workspace is locked by an active run".into()));
    }
    let runs: Vec<(String,)> = sqlx::query_as("SELECT id FROM runs WHERE workspace_id = ?").bind(&id).fetch_all(&state.db).await?;
    sqlx::query("DELETE FROM workspaces WHERE id = ?").bind(&id).execute(&state.db).await?;
    for (r,) in runs {
        let _ = tokio::fs::remove_dir_all(crate::runner::run_dir(&state, &r)).await;
    }
    // Only remove checkouts that Tuffie made. Never touch a user's local folder.
    if ws.source_kind == "git" {
        let _ = tokio::fs::remove_dir_all(state.config.checkouts_dir().join(&id)).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn sync(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let ws = get_workspace(&state, &id).await?;
    if ws.source_kind != "git" {
        return Err(bad("only git workspaces can be synced"));
    }
    if ws.lock_run_id.is_some() {
        return Err(ApiError::Conflict("cannot sync while a run is active".into()));
    }
    let dest = state.config.checkouts_dir().join(&id);
    let sha = git::sync(&dest, ws.git_url.as_deref().unwrap_or_default(), ws.git_ref.as_deref())
        .await
        .map_err(|e| bad(format!("{e:#}")))?;
    Ok(Json(json!({ "sha": sha })))
}
