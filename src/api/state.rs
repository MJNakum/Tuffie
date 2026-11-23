//! State viewer: current resources, dependency graph and state history.

use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};

use crate::error::{ApiError, ApiResult};
use crate::runner::{get_run, get_workspace};
use crate::{exec, AppState};

/// Flatten `values.root_module` and its child modules into one resource list.
fn flatten(module: &Value, out: &mut Vec<Value>) {
    if let Some(rs) = module["resources"].as_array() {
        for r in rs {
            out.push(json!({
                "address": r["address"],
                "module": module["address"],
                "mode": r["mode"],
                "type": r["type"],
                "name": r["name"],
                "index": r["index"],
                "provider_name": r["provider_name"],
                "values": super::mask_sensitive(&r["values"], Some(&r["sensitive_values"])),
                "depends_on": r["depends_on"],
                "tainted": r["tainted"].as_bool().unwrap_or(false),
            }));
        }
    }
    if let Some(children) = module["child_modules"].as_array() {
        for c in children {
            flatten(c, out);
        }
    }
}

fn graph(resources: &[Value]) -> Value {
    let mut edges = vec![];
    for r in resources {
        let from = r["address"].as_str().unwrap_or_default();
        if let Some(deps) = r["depends_on"].as_array() {
            for d in deps {
                if let Some(to) = d.as_str() {
                    edges.push(json!({ "from": from, "to": to }));
                }
            }
        }
    }
    json!({ "nodes": resources.iter().map(|r| json!({"id": r["address"], "type": r["type"], "mode": r["mode"]})).collect::<Vec<_>>(), "edges": edges })
}

fn summarize(show: &Value) -> Value {
    let mut resources = vec![];
    flatten(&show["values"]["root_module"], &mut resources);
    let outputs = show["values"]["outputs"].as_object().map(|o| {
        o.iter()
            .map(|(k, v)| {
                let sensitive = v["sensitive"].as_bool().unwrap_or(false);
                json!({
                    "name": k,
                    "sensitive": sensitive,
                    "type": v["type"],
                    "value": if sensitive { json!("(sensitive value)") } else { v["value"].clone() },
                })
            })
            .collect::<Vec<_>>()
    });
    json!({
        "terraform_version": show["terraform_version"],
        "graph": graph(&resources),
        "resources": resources,
        "outputs": outputs.unwrap_or_default(),
    })
}

/// Resource count cache file, written when the state is read.
pub async fn cached_resource_count(state: &AppState, ws_id: &str) -> Option<i64> {
    let p = state.config.data_dir.join("cache").join(format!("{ws_id}.count"));
    tokio::fs::read_to_string(p).await.ok()?.trim().parse().ok()
}

pub async fn current(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let ws = get_workspace(&state, &id).await?;
    let bin = exec::resolve_tool(&ws).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    let dir = exec::workdir(&state, &ws)?;
    if !dir.join(".terraform").is_dir() && !dir.join("terraform.tfstate").is_file() {
        return Ok(Json(json!({ "initialized": false, "resources": [], "outputs": [], "graph": {"nodes": [], "edges": []} })));
    }
    let env = exec::build_env(&state, &ws).await?;
    let out = tokio::process::Command::new(&bin)
        .args(["show", "-json", "-no-color"])
        .current_dir(&dir)
        .env_clear()
        .envs(&env.vars)
        .output()
        .await?;
    if !out.status.success() {
        let msg = env.redactor.redact(&String::from_utf8_lossy(&out.stderr));
        return Err(ApiError::BadRequest(format!("cannot read state: {}", msg.trim())));
    }
    let show: Value = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
    let mut s = summarize(&show);
    s["initialized"] = json!(true);
    let n = s["resources"].as_array().map(|a| a.iter().filter(|r| r["mode"] == "managed").count()).unwrap_or(0);
    super::write_resource_count(&state, &id, n).await;
    Ok(Json(s))
}

pub async fn versions(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let rows: Vec<(String, Option<i64>, chrono::DateTime<chrono::Utc>, String)> = sqlx::query_as(
        "SELECT id, state_serial, COALESCE(finished_at, created_at), kind FROM runs WHERE workspace_id = ? AND has_state_snapshot = 1 ORDER BY created_at DESC",
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(|(run_id, serial, at, kind)| json!({ "run_id": run_id, "serial": serial, "created_at": at, "kind": kind }))
        .collect::<Vec<_>>())))
}

/// Resources in the state snapshot saved after a run.
/// Raw state has a different shape from `show -json`, so it is mapped here.
pub async fn snapshot(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    get_run(&state, &id).await?;
    let path = crate::runner::run_dir(&state, &id).join("state.json.enc");
    if !path.exists() {
        return Err(ApiError::NotFound("this run has no state snapshot".into()));
    }
    let raw = state.crypto.read_encrypted(&path, &format!("{id}/state"))?;
    let st: Value = serde_json::from_slice(&raw).map_err(anyhow::Error::from)?;
    let mut resources = vec![];
    for r in st["resources"].as_array().into_iter().flatten() {
        let module = r["module"].as_str().unwrap_or("");
        let mode_prefix = if r["mode"] == "data" { "data." } else { "" };
        for inst in r["instances"].as_array().into_iter().flatten() {
            let base = format!("{}{}{}.{}", if module.is_empty() { String::new() } else { format!("{module}.") }, mode_prefix, r["type"].as_str().unwrap_or(""), r["name"].as_str().unwrap_or(""));
            let address = match &inst["index_key"] {
                Value::Null => base,
                Value::String(s) => format!("{base}[\"{s}\"]"),
                other => format!("{base}[{other}]"),
            };
            // Raw state lists sensitive paths in `sensitive_attributes`; to stay
            // safe we mask every attribute named there at the top level.
            let mut values = inst["attributes"].clone();
            for path in inst["sensitive_attributes"].as_array().into_iter().flatten() {
                if let Some(first) = path.as_array().and_then(|p| p.first()).and_then(|s| s["value"].as_str()) {
                    if let Some(o) = values.as_object_mut() {
                        if o.contains_key(first) {
                            o.insert(first.to_string(), json!("(sensitive value)"));
                        }
                    }
                }
            }
            resources.push(json!({
                "address": address,
                "module": module,
                "mode": r["mode"],
                "type": r["type"],
                "name": r["name"],
                "provider_name": r["provider"],
                "values": values,
                "depends_on": inst["dependencies"],
                "tainted": inst["status"] == "tainted",
            }));
        }
    }
    Ok(Json(json!({
        "serial": st["serial"],
        "lineage": st["lineage"],
        "terraform_version": st["terraform_version"],
        "graph": graph(&resources),
        "resources": resources,
        "outputs": st["outputs"].as_object().map(|o| o.iter().map(|(k, v)| {
            let sensitive = v["sensitive"].as_bool().unwrap_or(false);
            json!({"name": k, "sensitive": sensitive, "type": v["type"], "value": if sensitive { json!("(sensitive value)") } else { v["value"].clone() }})
        }).collect::<Vec<_>>()).unwrap_or_default(),
    })))
}
