use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::AppState;

pub async fn info(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "mode": state.config.mode,
        "auth": false,
        "data_dir": state.config.data_dir.display().to_string(),
        "inherit_env": state.config.inherit_env,
        "tools": crate::exec::detect_tools().await,
    }))
}
