//! Shared helpers to run `terraform` / `tofu` for a workspace:
//! binary lookup, working folder, environment and secret redaction.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, Context};
use serde::Serialize;

use crate::models::{VariableRow, Workspace};
use crate::AppState;

pub const TOOLS: [&str; 2] = ["terraform", "tofu"];

/// Find an executable on PATH.
pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join(name)).find(|p| is_executable(p))
}

fn is_executable(p: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        p.metadata().map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        p.is_file()
    }
}

pub fn resolve_tool(ws: &Workspace) -> anyhow::Result<PathBuf> {
    if let Some(p) = ws.tool_path.as_deref().filter(|p| !p.is_empty()) {
        let p = PathBuf::from(p);
        if is_executable(&p) {
            return Ok(p);
        }
        return Err(anyhow!("tool path {} is not an executable file", p.display()));
    }
    which(&ws.tool).ok_or_else(|| anyhow!("`{}` was not found on PATH; install it or set a tool path", ws.tool))
}

#[derive(Debug, Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub path: Option<String>,
    pub version: Option<String>,
}

pub async fn detect_tools() -> Vec<ToolInfo> {
    let mut out = vec![];
    for name in TOOLS {
        let path = which(name);
        let version = match &path {
            Some(p) => tool_version(p).await,
            None => None,
        };
        out.push(ToolInfo { name: name.into(), path: path.map(|p| p.display().to_string()), version });
    }
    out
}

pub async fn tool_version(bin: &Path) -> Option<String> {
    let out = tokio::process::Command::new(bin).args(["version", "-json"]).output().await.ok()?;
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    v.get("terraform_version").and_then(|s| s.as_str()).map(String::from)
}

/// Check that a user-supplied relative path stays inside its root.
pub fn safe_relative(p: &str) -> anyhow::Result<PathBuf> {
    let path = Path::new(p);
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::Normal(s) => out.push(s),
            Component::CurDir => {}
            _ => return Err(anyhow!("path must be relative and must not contain `..`")),
        }
    }
    Ok(out)
}

/// The root of the workspace source (local folder or git checkout).
pub fn source_root(state: &AppState, ws: &Workspace) -> anyhow::Result<PathBuf> {
    match ws.source_kind.as_str() {
        "local" => Ok(PathBuf::from(ws.local_path.as_deref().context("workspace has no local path")?)),
        "git" => Ok(state.config.checkouts_dir().join(&ws.id)),
        other => Err(anyhow!("unknown source kind {other}")),
    }
}

/// The folder where Terraform runs: source root + working directory.
pub fn workdir(state: &AppState, ws: &Workspace) -> anyhow::Result<PathBuf> {
    Ok(source_root(state, ws)?.join(safe_relative(&ws.working_dir)?))
}

/// Replaces secret values in output with `***`.
#[derive(Default, Clone)]
pub struct Redactor {
    secrets: Vec<String>,
}

impl Redactor {
    pub fn add(&mut self, s: &str) {
        // Very short values would redact too much ordinary text.
        if s.len() >= 4 {
            self.secrets.push(s.to_string());
        }
    }
    pub fn redact(&self, input: &str) -> String {
        let mut out = input.to_string();
        for s in &self.secrets {
            if out.contains(s.as_str()) {
                out = out.replace(s.as_str(), "***");
            }
        }
        out
    }
    pub fn is_empty(&self) -> bool {
        self.secrets.is_empty()
    }
}

/// Variables whose names are safe to pass from the host in server mode.
const SERVER_ENV_ALLOWLIST: &[&str] = &["PATH", "HOME", "USER", "LANG", "LC_ALL", "TZ", "TMPDIR", "SSL_CERT_FILE", "SSL_CERT_DIR"];

pub struct RunEnv {
    pub vars: BTreeMap<String, String>,
    pub redactor: Redactor,
}

pub async fn build_env(state: &AppState, ws: &Workspace) -> anyhow::Result<RunEnv> {
    let mut vars: BTreeMap<String, String> = if state.config.inherit_env {
        std::env::vars().filter(|(k, _)| !k.starts_with("TUFFIE_")).collect()
    } else {
        std::env::vars().filter(|(k, _)| SERVER_ENV_ALLOWLIST.contains(&k.as_str())).collect()
    };
    vars.insert("TF_IN_AUTOMATION".into(), "1".into());
    vars.insert("TF_INPUT".into(), "0".into());
    // Shared plugin cache makes `init` fast across workspaces.
    let cache = state.config.data_dir.join("plugin-cache");
    std::fs::create_dir_all(&cache).ok();
    vars.entry("TF_PLUGIN_CACHE_DIR".into()).or_insert_with(|| cache.display().to_string());

    let rows: Vec<VariableRow> = sqlx::query_as("SELECT * FROM variables WHERE workspace_id = ?")
        .bind(&ws.id)
        .fetch_all(&state.db)
        .await?;
    let mut redactor = Redactor::default();
    for row in rows {
        let value = state.crypto.decrypt(&row.value_enc, &row.id)?;
        if row.sensitive {
            redactor.add(&value);
        }
        let name = if row.category == "terraform" { format!("TF_VAR_{}", row.key) } else { row.key.clone() };
        vars.insert(name, value.to_string());
    }
    Ok(RunEnv { vars, redactor })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_escaping_paths() {
        assert!(safe_relative("../etc").is_err());
        assert!(safe_relative("/etc/passwd").is_err());
        assert!(safe_relative("a/../../b").is_err());
        assert_eq!(safe_relative("./envs/prod").unwrap(), PathBuf::from("envs/prod"));
    }

    #[test]
    fn redacts() {
        let mut r = Redactor::default();
        r.add("hunter22");
        r.add("ab"); // too short, ignored
        assert_eq!(r.redact("pw=hunter22 ab"), "pw=*** ab");
    }
}
