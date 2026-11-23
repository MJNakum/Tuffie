pub mod assign;
mod files;
mod policies;
mod varsets;
mod runs;
mod state;
mod system;
mod variables;
mod workspaces;

use axum::routing::{get, patch, post};
use axum::{middleware, Router};
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;

use crate::AppState;

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/system/info", get(system::info))
        .route("/workspaces", get(workspaces::list).post(workspaces::create))
        .route("/workspaces/sample", post(workspaces::create_demo))
        .route("/workspaces/{id}", get(workspaces::get).patch(workspaces::update).delete(workspaces::delete))
        .route("/workspaces/{id}/sync", post(workspaces::sync))
        .route("/workspaces/{id}/drift", post(workspaces::check_drift))
        .route("/workspaces/{id}/variable-sets", get(varsets::for_workspace))
        .route("/variable-sets", get(varsets::list).post(varsets::create))
        .route("/variable-sets/{id}", get(varsets::get).patch(varsets::update).delete(varsets::delete))
        .route("/variable-sets/{id}/variables", post(varsets::add_variable))
        .route("/policy-sets", get(policies::list).post(policies::create))
        .route("/policy-sets/testable-runs", get(policies::testable_runs))
        .route("/policy-sets/{id}", get(policies::get).patch(policies::update).delete(policies::delete))
        .route("/policy-sets/{id}/policies", post(policies::add_policy))
        .route("/policy-sets/{id}/test", post(policies::test))
        .route("/policies/{id}", patch(policies::update_policy).delete(policies::delete_policy))
        .route("/workspaces/{id}/runs", get(runs::list).post(runs::create))
        .route("/workspaces/{id}/variables", get(variables::list).post(variables::create))
        .route("/workspaces/{id}/files", get(files::tree))
        .route("/workspaces/{id}/files/content", get(files::read).put(files::write))
        .route("/workspaces/{id}/state", get(state::current))
        .route("/workspaces/{id}/state-versions", get(state::versions))
        .route("/workspaces/{id}/terminal", get(crate::terminal::ws))
        .route("/variables/{id}", patch(variables::update).delete(variables::delete))
        .route("/runs/{id}", get(runs::get))
        .route("/runs/{id}/events", get(runs::events))
        .route("/runs/{id}/stream", get(runs::stream))
        .route("/runs/{id}/plan", get(runs::plan))
        .route("/runs/{id}/output", get(runs::output))
        .route("/runs/{id}/policy", get(runs::policy))
        .route("/runs/{id}/variables", get(runs::variables))
        .route("/runs/{id}/state", get(state::snapshot))
        .route("/runs/{id}/apply", post(runs::apply))
        .route("/runs/{id}/discard", post(runs::discard))
        .route("/runs/{id}/cancel", post(runs::cancel));

    Router::new()
        .nest("/api", api)
        .fallback(crate::assets::serve)
        .layer(middleware::from_fn_with_state(state.clone(), crate::security::guard))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Cache the number of managed resources for the workspace list.
pub async fn write_resource_count(state: &crate::AppState, ws_id: &str, n: usize) {
    let cache = state.config.data_dir.join("cache");
    let _ = tokio::fs::create_dir_all(&cache).await;
    let _ = tokio::fs::write(cache.join(format!("{ws_id}.count")), n.to_string()).await;
}

/// Replace values that Terraform marks as sensitive.
///
/// `mask` has the same shape as `value`; `true` at a position means sensitive.
pub fn mask_sensitive(value: &serde_json::Value, mask: Option<&serde_json::Value>) -> serde_json::Value {
    use serde_json::Value;
    match mask {
        Some(Value::Bool(true)) => Value::String("(sensitive value)".into()),
        Some(Value::Object(m)) => match value {
            Value::Object(o) => {
                Value::Object(o.iter().map(|(k, v)| (k.clone(), mask_sensitive(v, m.get(k)))).collect())
            }
            other => other.clone(),
        },
        Some(Value::Array(m)) => match value {
            Value::Array(a) => Value::Array(a.iter().enumerate().map(|(i, v)| mask_sensitive(v, m.get(i))).collect()),
            other => other.clone(),
        },
        _ => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn masks_nested() {
        let v = json!({"a": "x", "b": {"c": "secret"}, "l": ["p", "q"]});
        let m = json!({"b": {"c": true}, "l": [false, true]});
        assert_eq!(
            mask_sensitive(&v, Some(&m)),
            json!({"a": "x", "b": {"c": "(sensitive value)"}, "l": ["p", "(sensitive value)"]})
        );
    }
}
