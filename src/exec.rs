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
    /// Where each variable came from. Keys only, never values.
    pub origins: Vec<VarOrigin>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct VarOrigin {
    pub key: String,
    pub category: String,
    /// `workspace` or `variable_set`.
    pub source: String,
    pub set_id: Option<String>,
    pub set_name: Option<String>,
    pub sensitive: bool,
    pub hcl: bool,
    /// True when a later source set the same variable.
    pub overridden: bool,
    /// The source that won, when `overridden`.
    pub overridden_by: Option<String>,
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

    // Precedence, lowest first: global variable sets, assigned variable sets
    // (each group by name), then the workspace's own variables.
    let mut layers: Vec<(Option<(String, String)>, Vec<VariableRow>)> = vec![];
    for set_id in crate::api::assign::sets_for_workspace(state, crate::api::assign::Kind::Variable, &ws.id).await? {
        let (name,): (String,) = sqlx::query_as("SELECT name FROM variable_sets WHERE id = ?").bind(&set_id).fetch_one(&state.db).await?;
        let rows: Vec<VariableRow> = sqlx::query_as("SELECT * FROM variables WHERE set_id = ? ORDER BY key").bind(&set_id).fetch_all(&state.db).await?;
        layers.push((Some((set_id, name)), rows));
    }
    let rows: Vec<VariableRow> = sqlx::query_as("SELECT * FROM variables WHERE workspace_id = ? ORDER BY key")
        .bind(&ws.id)
        .fetch_all(&state.db)
        .await?;
    layers.push((None, rows));

    let mut redactor = Redactor::default();
    let mut origins: Vec<VarOrigin> = vec![];
    for (set, rows) in layers {
        for row in rows {
            let value = state.crypto.decrypt(&row.value_enc, &row.id)?;
            if row.sensitive {
                redactor.add(&value);
            }
            let name = if row.category == "terraform" { format!("TF_VAR_{}", row.key) } else { row.key.clone() };
            let source_label = match &set {
                Some((_, n)) => format!("variable set {n}"),
                None => "workspace".to_string(),
            };
            for o in origins.iter_mut().filter(|o| o.key == row.key && o.category == row.category && !o.overridden) {
                o.overridden = true;
                o.overridden_by = Some(source_label.clone());
            }
            origins.push(VarOrigin {
                key: row.key.clone(),
                category: row.category.clone(),
                source: if set.is_some() { "variable_set".into() } else { "workspace".into() },
                set_id: set.as_ref().map(|s| s.0.clone()),
                set_name: set.as_ref().map(|s| s.1.clone()),
                sensitive: row.sensitive,
                hcl: row.hcl,
                overridden: false,
                overridden_by: None,
            });
            vars.insert(name, value.to_string());
        }
    }
    Ok(RunEnv { vars, redactor, origins })
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
