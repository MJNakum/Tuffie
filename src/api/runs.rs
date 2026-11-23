use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{bad, ApiError, ApiResult};
use crate::models::{status, Run, RunEvent, RunKind};
use crate::runner::{self, get_run, get_workspace};
use crate::AppState;

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    limit: i64,
}
fn default_limit() -> i64 {
    50
}

pub async fn list(State(state): State<AppState>, Path(id): Path<String>, Query(q): Query<ListQuery>) -> ApiResult<Json<Vec<Run>>> {
    let runs = sqlx::query_as("SELECT * FROM runs WHERE workspace_id = ? ORDER BY created_at DESC LIMIT ?")
        .bind(&id)
        .bind(q.limit.clamp(1, 500))
        .fetch_all(&state.db)
        .await?;
    Ok(Json(runs))
}

#[derive(Deserialize)]
pub struct CreateRun {
    kind: String,
    #[serde(default)]
    message: String,
}

pub async fn create(State(state): State<AppState>, Path(id): Path<String>, Json(req): Json<CreateRun>) -> ApiResult<(StatusCode, Json<Run>)> {
    let ws = get_workspace(&state, &id).await?;
    let kind = RunKind::parse(&req.kind).filter(|k| *k != RunKind::Cli).ok_or_else(|| bad("unknown run kind"))?;
    let message = if req.message.trim().is_empty() { "Triggered from the UI".to_string() } else { req.message };
    let run = runner::create_run(&state, &ws, kind, message, "ui").await?;
    Ok((StatusCode::CREATED, Json(run)))
}

pub async fn get(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let run = get_run(&state, &id).await?;
    let ws = get_workspace(&state, &run.workspace_id).await?;
    let position = if run.status == status::PENDING {
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM runs WHERE workspace_id = ? AND status = 'pending' AND created_at < ?")
            .bind(&run.workspace_id)
            .bind(run.created_at)
            .fetch_one(&state.db)
            .await?;
        Some(n)
    } else {
        None
    };
    let mut v = serde_json::to_value(&run).map_err(anyhow::Error::from)?;
    v["workspace_name"] = json!(ws.name);
    v["queue_position"] = json!(position);
    v["holds_lock"] = json!(ws.lock_run_id.as_deref() == Some(run.id.as_str()));
    Ok(Json(v))
}

async fn read_events(state: &AppState, id: &str) -> Vec<RunEvent> {
    match tokio::fs::read_to_string(runner::events_path(state, id)).await {
        Ok(s) => s.lines().filter_map(|l| serde_json::from_str(l).ok()).collect(),
        Err(_) => vec![],
    }
}

pub async fn events(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<Vec<RunEvent>>> {
    get_run(&state, &id).await?;
    Ok(Json(read_events(&state, &id).await))
}

/// Live events: send what is on disk, then new events as they happen.
pub async fn stream(State(state): State<AppState>, Path(id): Path<String>, ws: WebSocketUpgrade) -> ApiResult<Response> {
    get_run(&state, &id).await?;
    Ok(ws.on_upgrade(move |socket| stream_socket(state, id, socket)))
}

async fn stream_socket(state: AppState, id: String, mut socket: WebSocket) {
    // Subscribe first so no event is lost between the file read and the live feed.
    let mut rx = state.runs.subscribe(&id);
    let past = read_events(&state, &id).await;
    let mut last = None;
    for ev in past {
        last = Some(ev.seq);
        if send(&mut socket, &ev).await.is_err() {
            return;
        }
    }
    if let Some(rx) = rx.as_mut() {
        loop {
            match rx.recv().await {
                Ok(ev) => {
                    if last.is_some_and(|l| ev.seq <= l) {
                        continue;
                    }
                    if send(&mut socket, &ev).await.is_err() {
                        return;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    }
    let _ = socket.send(Message::Text(r#"{"type":"eof"}"#.into())).await;
}

async fn send(socket: &mut WebSocket, ev: &RunEvent) -> Result<(), axum::Error> {
    socket.send(Message::Text(serde_json::to_string(ev).unwrap_or_default().into())).await
}

/// The structured plan with sensitive values masked.
pub async fn plan(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    get_run(&state, &id).await?;
    let path = runner::run_dir(&state, &id).join("plan.json.enc");
    if !path.exists() {
        return Err(ApiError::NotFound("this run has no plan".into()));
    }
    let raw = state.crypto.read_encrypted(&path, &format!("{id}/plan"))?;
    let plan: Value = serde_json::from_slice(&raw).map_err(anyhow::Error::from)?;
    let changes = plan["resource_changes"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|rc| {
                    let c = &rc["change"];
                    json!({
                        "address": rc["address"],
                        "module_address": rc["module_address"],
                        "mode": rc["mode"],
                        "type": rc["type"],
                        "name": rc["name"],
                        "provider_name": rc["provider_name"],
                        "action_reason": rc["action_reason"],
                        "actions": c["actions"],
                        "before": super::mask_sensitive(&c["before"], Some(&c["before_sensitive"])),
                        "after": super::mask_sensitive(&c["after"], Some(&c["after_sensitive"])),
                        "after_unknown": c["after_unknown"],
                        "replace_paths": c["replace_paths"],
                        "importing": c["importing"],
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let outputs = plan["output_changes"]
        .as_object()
        .map(|o| {
            o.iter()
                .map(|(k, c)| {
                    json!({
                        "name": k,
                        "actions": c["actions"],
                        "before": super::mask_sensitive(&c["before"], Some(&c["before_sensitive"])),
                        "after": super::mask_sensitive(&c["after"], Some(&c["after_sensitive"])),
                        "after_unknown": c["after_unknown"],
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(Json(json!({
        "terraform_version": plan["terraform_version"],
        "resource_changes": changes,
        "output_changes": outputs,
        "resource_drift": plan["resource_drift"].as_array().map(|a| a.len()).unwrap_or(0),
        "drift": plan["resource_drift"].as_array().map(|a| a.iter().map(|rc| {
            let c = &rc["change"];
            json!({
                "address": rc["address"],
                "type": rc["type"],
                "mode": rc["mode"],
                "provider_name": rc["provider_name"],
                "actions": c["actions"],
                "before": super::mask_sensitive(&c["before"], Some(&c["before_sensitive"])),
                "after": super::mask_sensitive(&c["after"], Some(&c["after_sensitive"])),
            })
        }).collect::<Vec<_>>()).unwrap_or_default(),
    })))
}

#[derive(Deserialize, Default)]
pub struct ApplyReq {
    comment: Option<String>,
}

pub async fn apply(State(state): State<AppState>, Path(id): Path<String>, body: Option<Json<ApplyReq>>) -> ApiResult<StatusCode> {
    let comment = body.and_then(|b| b.0.comment);
    runner::confirm(&state, &id, comment).await.map_err(|e| ApiError::Conflict(e.to_string()))?;
    Ok(StatusCode::ACCEPTED)
}

pub async fn discard(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<StatusCode> {
    runner::discard(&state, &id).await.map_err(|e| ApiError::Conflict(e.to_string()))?;
    Ok(StatusCode::ACCEPTED)
}

pub async fn cancel(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<StatusCode> {
    runner::cancel(&state, &id).await.map_err(|e| ApiError::Conflict(e.to_string()))?;
    Ok(StatusCode::ACCEPTED)
}

/// Raw terminal output (with ANSI colors) for runs started from the terminal.
pub async fn output(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Response> {
    get_run(&state, &id).await?;
    let bytes = tokio::fs::read(runner::run_dir(&state, &id).join("output.log"))
        .await
        .map_err(|_| ApiError::NotFound("this run has no terminal output".into()))?;
    Ok(([(axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8")], bytes).into_response())
}

/// Policy check results of a run.
pub async fn policy(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    get_run(&state, &id).await?;
    let raw = tokio::fs::read(runner::run_dir(&state, &id).join("policy.json"))
        .await
        .map_err(|_| ApiError::NotFound("this run has no policy check".into()))?;
    Ok(Json(serde_json::from_slice(&raw).map_err(anyhow::Error::from)?))
}

/// Where each variable of the run came from (keys and sources, no values).
pub async fn variables(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    get_run(&state, &id).await?;
    match tokio::fs::read(runner::run_dir(&state, &id).join("variables.json")).await {
        Ok(raw) => Ok(Json(serde_json::from_slice(&raw).map_err(anyhow::Error::from)?)),
        Err(_) => Ok(Json(json!([]))),
    }
}
