//! File tree and editor API. Every path is checked to stay inside the
//! workspace source folder, including through symlinks.

use std::path::{Path as FsPath, PathBuf};

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::{bad, ApiError, ApiResult};
use crate::runner::get_workspace;
use crate::{exec, AppState};

const MAX_FILE: u64 = 2 * 1024 * 1024;
const SKIP_DIRS: &[&str] = &[".terraform", ".git", "node_modules", ".terragrunt-cache"];
const MAX_ENTRIES: usize = 5000;

/// State files hold secrets in plain text. The State tab shows them with
/// sensitive values masked, so the file editor never opens them.
fn is_state_file(name: &str) -> bool {
    name.ends_with(".tfstate") || name.ends_with(".tfstate.backup") || name.contains(".tfstate.")
}

#[derive(Serialize)]
pub struct Entry {
    path: String,
    name: String,
    dir: bool,
    size: u64,
    children: Option<Vec<Entry>>,
}

async fn root(state: &AppState, id: &str) -> ApiResult<PathBuf> {
    let ws = get_workspace(state, id).await?;
    let root = exec::source_root(state, &ws)?;
    Ok(tokio::fs::canonicalize(&root).await.map_err(|_| ApiError::NotFound("source folder not found".into()))?)
}

fn walk(root: &FsPath, dir: &FsPath, count: &mut usize) -> Vec<Entry> {
    let Ok(rd) = std::fs::read_dir(dir) else { return vec![] };
    let mut items: Vec<_> = rd.flatten().collect();
    items.sort_by_key(|e| (!e.path().is_dir(), e.file_name()));
    let mut out = vec![];
    for e in items {
        if *count >= MAX_ENTRIES {
            break;
        }
        let name = e.file_name().to_string_lossy().to_string();
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_symlink() || is_state_file(&name) {
            continue;
        }
        let path = e.path();
        let rel = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().to_string();
        *count += 1;
        if ft.is_dir() {
            if SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            let children = walk(root, &path, count);
            out.push(Entry { path: rel, name, dir: true, size: 0, children: Some(children) });
        } else {
            let size = e.metadata().map(|m| m.len()).unwrap_or(0);
            out.push(Entry { path: rel, name, dir: false, size, children: None });
        }
    }
    out
}

pub async fn tree(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<Vec<Entry>>> {
    let root = root(&state, &id).await?;
    let entries = tokio::task::spawn_blocking(move || {
        let mut n = 0;
        walk(&root, &root, &mut n)
    })
    .await
    .map_err(anyhow::Error::from)?;
    Ok(Json(entries))
}

#[derive(Deserialize)]
pub struct PathQuery {
    path: String,
}

/// Resolve a relative path and prove it stays inside `root`.
async fn resolve(root: &FsPath, rel: &str, must_exist: bool) -> ApiResult<PathBuf> {
    let rel = exec::safe_relative(rel).map_err(|e| bad(e.to_string()))?;
    if rel.as_os_str().is_empty() {
        return Err(bad("path is empty"));
    }
    if rel.components().any(|c| SKIP_DIRS.contains(&c.as_os_str().to_string_lossy().as_ref())) {
        return Err(ApiError::Forbidden("this folder cannot be edited".into()));
    }
    if rel.file_name().is_some_and(|n| is_state_file(&n.to_string_lossy())) {
        return Err(ApiError::Forbidden("state files cannot be opened here; use the State tab".into()));
    }
    let full = root.join(&rel);
    // Canonicalize the existing part to defeat symlinks that point outside.
    let check = if must_exist || full.exists() {
        tokio::fs::canonicalize(&full).await.map_err(|_| ApiError::NotFound("file not found".into()))?
    } else {
        let parent = full.parent().ok_or_else(|| bad("bad path"))?;
        let p = tokio::fs::canonicalize(parent).await.map_err(|_| bad("parent folder does not exist"))?;
        p.join(full.file_name().ok_or_else(|| bad("bad path"))?)
    };
    if !check.starts_with(root) {
        return Err(ApiError::Forbidden("path is outside the workspace".into()));
    }
    Ok(check)
}

#[derive(Serialize)]
pub struct FileContent {
    path: String,
    content: String,
    size: u64,
}

pub async fn read(State(state): State<AppState>, Path(id): Path<String>, Query(q): Query<PathQuery>) -> ApiResult<Json<FileContent>> {
    let root = root(&state, &id).await?;
    let full = resolve(&root, &q.path, true).await?;
    let meta = tokio::fs::metadata(&full).await?;
    if !meta.is_file() {
        return Err(bad("not a file"));
    }
    if meta.len() > MAX_FILE {
        return Err(bad("file is larger than 2 MB"));
    }
    let bytes = tokio::fs::read(&full).await?;
    let content = String::from_utf8(bytes).map_err(|_| bad("file is not UTF-8 text"))?;
    Ok(Json(FileContent { path: q.path, content, size: meta.len() }))
}

#[derive(Deserialize)]
pub struct WriteBody {
    content: String,
}

pub async fn write(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<PathQuery>,
    Json(body): Json<WriteBody>,
) -> ApiResult<StatusCode> {
    if body.content.len() as u64 > MAX_FILE {
        return Err(bad("file is larger than 2 MB"));
    }
    let root = root(&state, &id).await?;
    let full = resolve(&root, &q.path, false).await?;
    // Write to a temp file and rename, so a crash never leaves half a file.
    let tmp = full.with_extension(format!("tuffie-tmp-{}", std::process::id()));
    tokio::fs::write(&tmp, body.content.as_bytes()).await?;
    tokio::fs::rename(&tmp, &full).await?;
    Ok(StatusCode::NO_CONTENT)
}
