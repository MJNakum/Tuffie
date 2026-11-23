//! Run engine: queue, workspace lock, plan → confirm → apply.
//!
//! One run at a time holds a workspace lock. A planned run that waits for
//! confirmation keeps the lock (same as HCP Terraform), so nothing changes
//! state between the plan you review and the apply.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::broadcast;

use crate::exec::{self, Redactor};
use crate::models::{now, status, Run, RunEvent, RunKind, Workspace};
use crate::AppState;

pub struct RunHub {
    channels: Mutex<HashMap<String, Channel>>,
    /// PIDs of running processes, for cancel.
    pids: Mutex<HashMap<String, u32>>,
}

struct Channel {
    tx: broadcast::Sender<RunEvent>,
    seq: Arc<AtomicU64>,
}

impl RunHub {
    pub fn new() -> Self {
        Self { channels: Mutex::new(HashMap::new()), pids: Mutex::new(HashMap::new()) }
    }

    pub fn subscribe(&self, run_id: &str) -> Option<broadcast::Receiver<RunEvent>> {
        self.channels.lock().unwrap().get(run_id).map(|c| c.tx.subscribe())
    }

    fn open(&self, run_id: &str, start_seq: u64) -> (broadcast::Sender<RunEvent>, Arc<AtomicU64>) {
        let mut map = self.channels.lock().unwrap();
        let ch = map.entry(run_id.to_string()).or_insert_with(|| Channel {
            tx: broadcast::channel(4096).0,
            seq: Arc::new(AtomicU64::new(start_seq)),
        });
        (ch.tx.clone(), ch.seq.clone())
    }

    fn close(&self, run_id: &str) {
        self.channels.lock().unwrap().remove(run_id);
    }
}

pub fn run_dir(state: &AppState, run_id: &str) -> PathBuf {
    state.config.runs_dir().join(run_id)
}

pub fn events_path(state: &AppState, run_id: &str) -> PathBuf {
    run_dir(state, run_id).join("events.jsonl")
}

/// Writes events to disk and to live subscribers.
pub struct EventSink {
    run_id: String,
    stage: String,
    file: tokio::fs::File,
    tx: broadcast::Sender<RunEvent>,
    seq: Arc<AtomicU64>,
    redactor: Redactor,
}

impl EventSink {
    pub async fn open(state: &AppState, run_id: &str, redactor: Redactor) -> anyhow::Result<Self> {
        let dir = run_dir(state, run_id);
        tokio::fs::create_dir_all(&dir).await?;
        let path = events_path(state, run_id);
        let existing = match tokio::fs::read_to_string(&path).await {
            Ok(s) => s.lines().count() as u64,
            Err(_) => 0,
        };
        let file = tokio::fs::OpenOptions::new().create(true).append(true).open(&path).await?;
        let (tx, seq) = state.runs.open(run_id, existing);
        Ok(Self { run_id: run_id.into(), stage: "system".into(), file, tx, seq, redactor })
    }

    pub fn stage(&mut self, s: &str) {
        self.stage = s.into();
    }

    pub async fn emit(&mut self, level: &str, message: impl AsRef<str>, ty: Option<&str>, data: Option<serde_json::Value>) {
        let data = data.map(|d| {
            if self.redactor.is_empty() {
                d
            } else {
                serde_json::from_str(&self.redactor.redact(&d.to_string())).unwrap_or(serde_json::Value::Null)
            }
        });
        let ev = RunEvent {
            seq: self.seq.fetch_add(1, Ordering::SeqCst),
            ts: now(),
            stage: self.stage.clone(),
            level: level.into(),
            message: self.redactor.redact(message.as_ref()),
            r#type: ty.map(String::from),
            data,
        };
        if let Ok(mut line) = serde_json::to_string(&ev) {
            line.push('\n');
            if let Err(e) = self.file.write_all(line.as_bytes()).await {
                tracing::warn!(run = %self.run_id, error = %e, "cannot write run event");
            }
        }
        let _ = self.tx.send(ev);
    }

    pub async fn info(&mut self, m: impl AsRef<str>) {
        self.emit("info", m, None, None).await
    }
    pub async fn error(&mut self, m: impl AsRef<str>) {
        self.emit("error", m, None, None).await
    }
    pub async fn status(&mut self, s: &str) {
        self.emit("info", format!("Run status: {s}"), Some("status"), Some(serde_json::json!({ "status": s }))).await
    }
}

// ---------- database helpers ----------

pub async fn get_run(state: &AppState, id: &str) -> Result<Run, sqlx::Error> {
    sqlx::query_as("SELECT * FROM runs WHERE id = ?").bind(id).fetch_one(&state.db).await
}

pub async fn get_workspace(state: &AppState, id: &str) -> Result<Workspace, sqlx::Error> {
    sqlx::query_as("SELECT * FROM workspaces WHERE id = ?").bind(id).fetch_one(&state.db).await
}

async fn set_status(state: &AppState, run_id: &str, s: &str) -> anyhow::Result<()> {
    let finished = if status::is_final(s) { Some(now()) } else { None };
    sqlx::query("UPDATE runs SET status = ?, finished_at = COALESCE(?, finished_at) WHERE id = ?")
        .bind(s)
        .bind(finished)
        .bind(run_id)
        .execute(&state.db)
        .await?;
    Ok(())
}

/// Take the workspace lock for a run. Returns false when another run holds it.
pub async fn try_lock(state: &AppState, ws_id: &str, run_id: &str) -> anyhow::Result<bool> {
    let r = sqlx::query("UPDATE workspaces SET lock_run_id = ? WHERE id = ? AND lock_run_id IS NULL")
        .bind(run_id)
        .bind(ws_id)
        .execute(&state.db)
        .await?;
    Ok(r.rows_affected() == 1)
}

pub async fn unlock(state: &AppState, ws_id: &str, run_id: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE workspaces SET lock_run_id = NULL WHERE id = ? AND lock_run_id = ?")
        .bind(ws_id)
        .bind(run_id)
        .execute(&state.db)
        .await?;
    Ok(())
}

// ---------- public operations ----------

pub async fn create_run(state: &AppState, ws: &Workspace, kind: RunKind, message: String) -> anyhow::Result<Run> {
    let id = crate::models::new_id("run");
    sqlx::query("INSERT INTO runs (id, workspace_id, kind, source, status, message, created_at) VALUES (?, ?, ?, 'ui', ?, ?, ?)")
        .bind(&id)
        .bind(&ws.id)
        .bind(kind.as_str())
        .bind(status::PENDING)
        .bind(&message)
        .bind(now())
        .execute(&state.db)
        .await?;
    let state2 = state.clone();
    let ws_id = ws.id.clone();
    tokio::spawn(async move { kick(&state2, &ws_id).await });
    Ok(get_run(state, &id).await?)
}

/// Start the oldest pending run of a workspace if the lock is free.
pub async fn kick(state: &AppState, ws_id: &str) {
    kick_boxed(state.clone(), ws_id.to_string()).await
}

// Boxed with an explicit `Send` bound: kick -> run -> finish -> kick is a cycle,
// and the compiler cannot infer `Send` through a cycle of opaque futures.
fn kick_boxed(state: AppState, ws_id: String) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
    Box::pin(async move {
        if let Err(e) = kick_inner(&state, &ws_id).await {
            tracing::error!(workspace = %ws_id, error = ?e, "cannot start queued run");
        }
    })
}

async fn kick_inner(state: &AppState, ws_id: &str) -> anyhow::Result<()> {
    let next: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM runs WHERE workspace_id = ? AND status = ? AND kind != 'cli' ORDER BY created_at ASC LIMIT 1",
    )
    .bind(ws_id)
    .bind(status::PENDING)
    .fetch_optional(&state.db)
    .await?;
    let Some((run_id,)) = next else { return Ok(()) };
    if !try_lock(state, ws_id, &run_id).await? {
        return Ok(());
    }
    let state = state.clone();
    tokio::spawn(async move {
        let ws_id = {
            let r = get_run(&state, &run_id).await;
            r.map(|r| r.workspace_id).unwrap_or_default()
        };
        if let Err(e) = execute_plan(&state, &run_id).await {
            tracing::error!(run = %run_id, error = ?e, "run failed");
            fail(&state, &run_id, &format!("{e:#}")).await;
        }
        // Release the lock unless the run waits for confirmation.
        if let Ok(run) = get_run(&state, &run_id).await {
            if run.status != status::PLANNED {
                finish(&state, &ws_id, &run_id).await;
            }
        }
    });
    Ok(())
}

async fn finish(state: &AppState, ws_id: &str, run_id: &str) {
    let _ = unlock(state, ws_id, run_id).await;
    state.runs.close(run_id);
    kick_boxed(state.clone(), ws_id.to_string()).await;
}

async fn fail(state: &AppState, run_id: &str, msg: &str) {
    if let Ok(mut sink) = EventSink::open(state, run_id, Redactor::default()).await {
        sink.error(msg).await;
        sink.status(status::ERRORED).await;
    }
    let _ = set_status(state, run_id, status::ERRORED).await;
}

pub async fn confirm(state: &AppState, run_id: &str, comment: Option<String>) -> anyhow::Result<()> {
    let r = sqlx::query("UPDATE runs SET status = ?, confirmed_comment = ? WHERE id = ? AND status = ?")
        .bind(status::APPLYING)
        .bind(comment)
        .bind(run_id)
        .bind(status::PLANNED)
        .execute(&state.db)
        .await?;
    if r.rows_affected() != 1 {
        return Err(anyhow!("only a planned run that waits for confirmation can be applied"));
    }
    let state = state.clone();
    let run_id = run_id.to_string();
    tokio::spawn(async move {
        let ws_id = get_run(&state, &run_id).await.map(|r| r.workspace_id).unwrap_or_default();
        if let Err(e) = execute_apply(&state, &run_id).await {
            fail(&state, &run_id, &format!("{e:#}")).await;
        }
        finish(&state, &ws_id, &run_id).await;
    });
    Ok(())
}

pub async fn discard(state: &AppState, run_id: &str) -> anyhow::Result<()> {
    let run = get_run(state, run_id).await?;
    if run.status != status::PLANNED {
        return Err(anyhow!("only a planned run can be discarded"));
    }
    set_status(state, run_id, status::DISCARDED).await?;
    let _ = tokio::fs::remove_file(run_dir(state, run_id).join("plan.tfplan")).await;
    let mut sink = EventSink::open(state, run_id, Redactor::default()).await?;
    sink.status(status::DISCARDED).await;
    finish(state, &run.workspace_id, run_id).await;
    Ok(())
}

pub async fn cancel(state: &AppState, run_id: &str) -> anyhow::Result<()> {
    let run = get_run(state, run_id).await?;
    match run.status.as_str() {
        status::PENDING => {
            set_status(state, run_id, status::CANCELED).await?;
            Ok(())
        }
        status::PLANNING | status::APPLYING | status::RUNNING => {
            let pid = state.runs.pids.lock().unwrap().get(run_id).copied();
            match pid {
                // SIGINT lets Terraform stop cleanly and release its state lock.
                Some(pid) => unsafe {
                    libc::kill(pid as i32, libc::SIGINT);
                },
                None => return Err(anyhow!("the run has no running process")),
            }
            let mut sink = EventSink::open(state, run_id, Redactor::default()).await?;
            sink.info("Cancel requested; sent interrupt to Terraform").await;
            state.canceled.lock().unwrap().insert(run_id.to_string());
            Ok(())
        }
        _ => Err(anyhow!("run is not active")),
    }
}

/// On start, runs that were active when the server stopped cannot continue.
pub async fn recover(state: &AppState) -> anyhow::Result<()> {
    let active: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, workspace_id FROM runs WHERE status IN ('planning', 'applying', 'running')",
    )
    .fetch_all(&state.db)
    .await?;
    for (id, ws) in active {
        fail(state, &id, "Tuffie stopped while this run was active").await;
        let _ = unlock(state, &ws, &id).await;
    }
    let ws: Vec<(String,)> = sqlx::query_as("SELECT id FROM workspaces").fetch_all(&state.db).await?;
    for (id,) in ws {
        kick(state, &id).await;
    }
    Ok(())
}

// ---------- execution ----------

struct Ctx {
    bin: PathBuf,
    dir: PathBuf,
    rundir: PathBuf,
    env: std::collections::BTreeMap<String, String>,
}

async fn prepare(state: &AppState, run: &Run, sink: &mut EventSink) -> anyhow::Result<Ctx> {
    let ws = get_workspace(state, &run.workspace_id).await?;
    let bin = exec::resolve_tool(&ws)?;
    let dir = exec::workdir(state, &ws)?;
    if !dir.is_dir() {
        return Err(anyhow!("working folder {} does not exist", dir.display()));
    }
    let env = exec::build_env(state, &ws).await?;
    sink.redactor = env.redactor;
    sink.info(format!("Using {} in {}", bin.display(), dir.display())).await;
    Ok(Ctx { bin, dir, rundir: run_dir(state, &run.id), env: env.vars })
}

async fn execute_plan(state: &AppState, run_id: &str) -> anyhow::Result<()> {
    let run = get_run(state, run_id).await?;
    let kind = RunKind::parse(&run.kind).context("bad run kind")?;
    let mut sink = EventSink::open(state, run_id, Redactor::default()).await?;
    sqlx::query("UPDATE runs SET status = ?, started_at = ? WHERE id = ?")
        .bind(status::PLANNING)
        .bind(now())
        .bind(run_id)
        .execute(&state.db)
        .await?;
    sink.status(status::PLANNING).await;
    let ctx = prepare(state, &run, &mut sink).await?;

    // init
    sink.stage("init");
    let code = run_tool(state, run_id, &ctx, &["init", "-input=false", "-no-color"], false, &mut sink).await?;
    if code != 0 {
        return Err(anyhow!("init failed (exit code {code})"));
    }
    if cancelled(state, run_id) {
        return mark_canceled(state, run_id, &mut sink).await;
    }

    // plan
    sink.stage("plan");
    let plan_file = ctx.rundir.join("plan.tfplan");
    let out_arg = format!("-out={}", plan_file.display());
    let mut args = vec!["plan", "-input=false", "-json", "-detailed-exitcode", out_arg.as_str()];
    match kind {
        RunKind::Destroy => args.push("-destroy"),
        RunKind::Refresh => args.push("-refresh-only"),
        _ => {}
    }
    let code = run_tool(state, run_id, &ctx, &args, true, &mut sink).await?;
    if cancelled(state, run_id) {
        return mark_canceled(state, run_id, &mut sink).await;
    }
    if code == 1 || !(code == 0 || code == 2) {
        return Err(anyhow!("plan failed (exit code {code})"));
    }
    let has_changes = code == 2;

    // Structured plan for the UI. The file can contain secrets, so it is private.
    let show = tokio::process::Command::new(&ctx.bin)
        .args(["show", "-json"])
        .arg(&plan_file)
        .current_dir(&ctx.dir)
        .env_clear()
        .envs(&ctx.env)
        .output()
        .await?;
    if show.status.success() {
        state.crypto.write_encrypted(&ctx.rundir.join("plan.json.enc"), &show.stdout, &format!("{run_id}/plan"))?;
    } else {
        sink.emit("warn", "Could not export the plan as JSON", None, None).await;
    }

    sqlx::query("UPDATE runs SET has_changes = ?, plan_finished_at = ? WHERE id = ?")
        .bind(has_changes)
        .bind(now())
        .bind(run_id)
        .execute(&state.db)
        .await?;

    let ws = get_workspace(state, &run.workspace_id).await?;
    if !has_changes || kind == RunKind::PlanOnly {
        // The binary plan file holds secrets; keep it only while it can be applied.
        let _ = tokio::fs::remove_file(&plan_file).await;
        set_status(state, run_id, status::PLANNED_AND_FINISHED).await?;
        sink.status(status::PLANNED_AND_FINISHED).await;
        return Ok(());
    }
    set_status(state, run_id, status::PLANNED).await?;
    sink.status(status::PLANNED).await;
    if ws.auto_apply {
        sink.info("Auto-apply is on for this workspace").await;
        drop(sink);
        // Keep the lock and go straight to apply.
        sqlx::query("UPDATE runs SET status = ?, confirmed_comment = 'auto-apply' WHERE id = ?")
            .bind(status::APPLYING)
            .bind(run_id)
            .execute(&state.db)
            .await?;
        execute_apply(state, run_id).await?;
    }
    Ok(())
}

async fn execute_apply(state: &AppState, run_id: &str) -> anyhow::Result<()> {
    let run = get_run(state, run_id).await?;
    let mut sink = EventSink::open(state, run_id, Redactor::default()).await?;
    sink.status(status::APPLYING).await;
    let ctx = prepare(state, &run, &mut sink).await?;
    sink.stage("apply");
    let plan_file = ctx.rundir.join("plan.tfplan");
    let plan_arg = plan_file.display().to_string();
    let code = run_tool(state, run_id, &ctx, &["apply", "-input=false", "-json", plan_arg.as_str()], true, &mut sink).await?;
    // Plan files can hold secrets; remove after use.
    let _ = tokio::fs::remove_file(&plan_file).await;
    snapshot_state(state, run_id, &ctx, &mut sink).await;
    if code != 0 {
        return Err(anyhow!("apply failed (exit code {code})"));
    }
    set_status(state, run_id, status::APPLIED).await?;
    sink.status(status::APPLIED).await;
    Ok(())
}

/// Save a copy of the state after apply. This gives state history for any backend.
async fn snapshot_state(state: &AppState, run_id: &str, ctx: &Ctx, sink: &mut EventSink) {
    let out = tokio::process::Command::new(&ctx.bin)
        .args(["state", "pull"])
        .current_dir(&ctx.dir)
        .env_clear()
        .envs(&ctx.env)
        .output()
        .await;
    match out {
        Ok(o) if o.status.success() && !o.stdout.is_empty() => {
            let serial = serde_json::from_slice::<serde_json::Value>(&o.stdout)
                .ok()
                .and_then(|v| v.get("serial").and_then(|s| s.as_i64()));
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&o.stdout) {
                let n: usize = v["resources"]
                    .as_array()
                    .map(|rs| {
                        rs.iter()
                            .filter(|r| r["mode"] == "managed")
                            .map(|r| r["instances"].as_array().map(|i| i.len()).unwrap_or(0))
                            .sum()
                    })
                    .unwrap_or(0);
                crate::api::write_resource_count(state, &run_ws(state, run_id).await, n).await;
            }
            if state.crypto.write_encrypted(&ctx.rundir.join("state.json.enc"), &o.stdout, &format!("{run_id}/state")).is_ok() {
                let _ = sqlx::query("UPDATE runs SET has_state_snapshot = 1, state_serial = ? WHERE id = ?")
                    .bind(serial)
                    .bind(run_id)
                    .execute(&state.db)
                    .await;
            }
        }
        _ => sink.emit("warn", "Could not save a state snapshot", None, None).await,
    }
}

async fn run_ws(state: &AppState, run_id: &str) -> String {
    get_run(state, run_id).await.map(|r| r.workspace_id).unwrap_or_default()
}

fn cancelled(state: &AppState, run_id: &str) -> bool {
    state.canceled.lock().unwrap().contains(run_id)
}

async fn mark_canceled(state: &AppState, run_id: &str, sink: &mut EventSink) -> anyhow::Result<()> {
    state.canceled.lock().unwrap().remove(run_id);
    set_status(state, run_id, status::CANCELED).await?;
    sink.status(status::CANCELED).await;
    Ok(())
}

/// Run the tool and stream its output as events. Returns the exit code.
async fn run_tool(
    state: &AppState,
    run_id: &str,
    ctx: &Ctx,
    args: &[&str],
    json: bool,
    sink: &mut EventSink,
) -> anyhow::Result<i32> {
    sink.emit("info", format!("$ {} {}", tool_name(&ctx.bin), args.join(" ")), Some("command"), None).await;
    let mut child = tokio::process::Command::new(&ctx.bin)
        .args(args)
        .current_dir(&ctx.dir)
        .env_clear()
        .envs(&ctx.env)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("cannot start {}", ctx.bin.display()))?;
    if let Some(pid) = child.id() {
        state.runs.pids.lock().unwrap().insert(run_id.to_string(), pid);
    }
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<(bool, String)>(256);
    let tx2 = tx.clone();
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            if tx.send((false, l)).await.is_err() {
                break;
            }
        }
    });
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            if tx2.send((true, l)).await.is_err() {
                break;
            }
        }
    });
    while let Some((is_err, line)) = rx.recv().await {
        if json {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                handle_json(state, run_id, &v, sink).await;
                continue;
            }
        }
        if line.trim().is_empty() {
            continue;
        }
        sink.emit(if is_err { "error" } else { "info" }, &line, None, None).await;
    }
    let status = child.wait().await?;
    state.runs.pids.lock().unwrap().remove(run_id);
    Ok(status.code().unwrap_or(-1))
}

fn tool_name(bin: &Path) -> String {
    bin.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

async fn handle_json(state: &AppState, run_id: &str, v: &serde_json::Value, sink: &mut EventSink) {
    let ty = v.get("type").and_then(|t| t.as_str()).unwrap_or("log").to_string();
    let level = v.get("@level").and_then(|t| t.as_str()).unwrap_or("info").to_string();
    let msg = v.get("@message").and_then(|t| t.as_str()).unwrap_or_default().to_string();
    if ty == "change_summary" {
        if let Some(c) = v.get("changes") {
            let g = |k: &str| c.get(k).and_then(|x| x.as_i64()).unwrap_or(0);
            let op = c.get("operation").and_then(|x| x.as_str()).unwrap_or("plan");
            if op == "plan" {
                let _ = sqlx::query("UPDATE runs SET add_count = ?, change_count = ?, destroy_count = ?, import_count = ? WHERE id = ?")
                    .bind(g("add"))
                    .bind(g("change"))
                    .bind(g("remove"))
                    .bind(g("import"))
                    .bind(run_id)
                    .execute(&state.db)
                    .await;
            }
        }
    }
    // Keep only the useful fields of the JSON message.
    let data = match ty.as_str() {
        "version" | "log" => None,
        _ => Some(v.clone()),
    };
    sink.emit(&level, msg, Some(&ty), data).await;
}
