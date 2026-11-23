//! Git sources: clone and sync with the system `git` binary.
//! Credentials come from the user's normal git setup (SSH agent, credential helper).

use std::path::Path;

use anyhow::{anyhow, Context};

pub fn validate_url(url: &str) -> anyhow::Result<()> {
    let ok = url.starts_with("https://")
        || url.starts_with("ssh://")
        || url.starts_with("git@")
        || url.starts_with("file://");
    if !ok || url.contains(char::is_whitespace) {
        return Err(anyhow!("git URL must start with https://, ssh://, git@ or file://"));
    }
    Ok(())
}

pub fn validate_ref(r: &str) -> anyhow::Result<()> {
    if r.starts_with('-') || r.contains("..") || r.contains(char::is_whitespace) {
        return Err(anyhow!("invalid git ref"));
    }
    Ok(())
}

async fn git(dir: Option<&Path>, args: &[&str]) -> anyhow::Result<String> {
    let mut cmd = tokio::process::Command::new("git");
    // Block transports that can run commands.
    cmd.args(["-c", "protocol.ext.allow=never", "-c", "core.hooksPath=/dev/null"]);
    cmd.args(args);
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    let out = cmd.output().await.context("cannot run git; is it installed?")?;
    if !out.status.success() {
        return Err(anyhow!("git {}: {}", args.first().unwrap_or(&""), String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Clone if missing, then fetch and check out the ref. Returns the commit SHA.
pub async fn sync(dest: &Path, url: &str, git_ref: Option<&str>) -> anyhow::Result<String> {
    validate_url(url)?;
    if !dest.join(".git").exists() {
        tokio::fs::create_dir_all(dest.parent().unwrap_or(dest)).await?;
        let d = dest.display().to_string();
        git(None, &["clone", "--quiet", "--", url, &d]).await?;
    }
    git(Some(dest), &["fetch", "--quiet", "--prune", "origin"]).await?;
    let target = match git_ref.filter(|r| !r.is_empty()) {
        Some(r) => {
            validate_ref(r)?;
            // Prefer the remote branch; fall back to a tag or SHA.
            let remote = format!("origin/{r}");
            if git(Some(dest), &["rev-parse", "--verify", "--quiet", &remote]).await.is_ok() {
                remote
            } else {
                r.to_string()
            }
        }
        None => "origin/HEAD".to_string(),
    };
    git(Some(dest), &["checkout", "--quiet", "--force", "--detach", &target]).await?;
    git(Some(dest), &["rev-parse", "HEAD"]).await
}

pub async fn head(dest: &Path) -> Option<String> {
    git(Some(dest), &["log", "-1", "--format=%H%x00%s%x00%an%x00%cI"]).await.ok()
}
