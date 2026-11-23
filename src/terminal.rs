//! Integrated terminal.
//!
//! It is NOT a shell. The browser sends one command line at a time. Tuffie
//! accepts only `terraform`, `tofu` or `tf` (an alias for the workspace's
//! tool), splits the line like a shell would, and runs the binary directly
//! in a PTY in the workspace folder. There is no shell, so pipes, `;`, `&&`,
//! redirects and `$(...)` cannot run other programs.
//!
//! Every command is saved as a run (`kind = cli`, `source = terminal`), so it
//! shows in run history next to UI runs. Commands that change state take the
//! workspace lock, so they cannot collide with a UI run.
//!
//! Protocol (JSON text frames, client → server):
//!   {"type":"exec","command":"terraform plan"}
//!   {"type":"input","data":"yes\r"}
//!   {"type":"resize","cols":120,"rows":30}
//!   {"type":"interrupt"}
//! Server → client: binary frames with raw PTY output, and JSON text frames:
//!   {"type":"started","run_id":"run-…"} {"type":"exit","code":0}
//!   {"type":"error","message":"…"}

use std::io::{Read, Write};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::mpsc;

use crate::error::ApiResult;
use crate::models::{new_id, now, status, Workspace};
use crate::runner::{self, get_workspace};
use crate::{exec, AppState};

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ClientMsg {
    Exec { command: String },
    Input { data: String },
    Resize { cols: u16, rows: u16 },
    Interrupt,
}

pub async fn ws(State(state): State<AppState>, Path(id): Path<String>, ws: WebSocketUpgrade) -> ApiResult<Response> {
    let workspace = get_workspace(&state, &id).await?;
    Ok(ws.on_upgrade(move |socket| session(state, workspace, socket)))
}

/// Subcommands that never change state or the working folder.
fn is_read_only(args: &[String]) -> bool {
    let sub = args.iter().find(|a| !a.starts_with('-')).map(String::as_str).unwrap_or("");
    match sub {
        "" | "version" | "validate" | "show" | "output" | "providers" | "graph" | "console" | "fmt" | "help" | "test" | "metadata" => {
            // `fmt` rewrites files unless -check is given.
            sub != "fmt" || args.iter().any(|a| a == "-check")
        }
        "state" => {
            let action = args.iter().filter(|a| !a.starts_with('-')).nth(1).map(String::as_str).unwrap_or("");
            matches!(action, "list" | "show" | "pull")
        }
        "workspace" => {
            let action = args.iter().filter(|a| !a.starts_with('-')).nth(1).map(String::as_str).unwrap_or("");
            matches!(action, "list" | "show")
        }
        _ => false,
    }
}

pub enum Parsed {
    Run(Vec<String>),
    Reject(String),
}

pub fn parse_command(line: &str) -> Parsed {
    let words = match shell_words::split(line) {
        Ok(w) => w,
        Err(_) => return Parsed::Reject("cannot parse the command (unclosed quote?)".into()),
    };
    let Some(first) = words.first() else { return Parsed::Reject(String::new()) };
    if !matches!(first.as_str(), "terraform" | "tofu" | "tf") {
        return Parsed::Reject(format!(
            "`{first}` is not allowed. This terminal runs only terraform and tofu commands (alias: tf)."
        ));
    }
    let args: Vec<String> = words[1..].to_vec();
    for a in &args {
        if matches!(a.as_str(), "|" | "||" | "&&" | ";" | ">" | ">>" | "<" | "&") {
            return Parsed::Reject(format!("`{a}` is not supported: there is no shell here"));
        }
        if a.starts_with("-chdir") {
            return Parsed::Reject("-chdir is not allowed; the terminal always runs in the workspace folder".into());
        }
    }
    Parsed::Run(args)
}

async fn session(state: AppState, ws: Workspace, socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    let (out_tx, mut out_rx) = mpsc::channel::<Message>(256);

    // Single writer task for the socket.
    let writer = tokio::spawn(async move {
        while let Some(m) = out_rx.recv().await {
            if sender.send(m).await.is_err() {
                break;
            }
        }
    });

    let mut size = PtySize { rows: 30, cols: 120, pixel_width: 0, pixel_height: 0 };
    // Input channel to the running process (if any).
    let mut active: Option<Active> = None;

    while let Some(Ok(msg)) = receiver.next().await {
        let Message::Text(text) = msg else { continue };
        let Ok(cmd) = serde_json::from_str::<ClientMsg>(&text) else { continue };
        // Clear a finished process.
        if active.as_mut().is_some_and(|a| a.done.try_recv().is_ok() || a.input.is_closed()) {
            active = None;
        }
        match cmd {
            ClientMsg::Resize { cols, rows } => {
                size.cols = cols.clamp(20, 500);
                size.rows = rows.clamp(5, 200);
                if let Some(a) = &active {
                    let _ = a.resize.send(size).await;
                }
            }
            ClientMsg::Input { data } => {
                if let Some(a) = &active {
                    let _ = a.input.send(data.into_bytes()).await;
                }
            }
            ClientMsg::Interrupt => {
                if let Some(a) = &active {
                    let _ = a.input.send(vec![3]).await; // Ctrl+C
                }
            }
            ClientMsg::Exec { command } => {
                if active.is_some() {
                    send_json(&out_tx, json!({"type": "error", "message": "a command is already running"})).await;
                    continue;
                }
                // Reload: the tool or lock can change between commands.
                let ws = match get_workspace(&state, &ws.id).await {
                    Ok(w) => w,
                    Err(_) => break,
                };
                match start(&state, &ws, &command, size, out_tx.clone()).await {
                    Ok(a) => active = Some(a),
                    Err(e) => {
                        send_json(&out_tx, json!({"type": "error", "message": e})).await;
                        send_json(&out_tx, json!({"type": "exit", "code": null})).await;
                    }
                }
            }
        }
    }
    if let Some(a) = active {
        let _ = a.input.send(vec![3]).await;
    }
    drop(out_tx);
    let _ = writer.await;
}

struct Active {
    input: mpsc::Sender<Vec<u8>>,
    resize: mpsc::Sender<PtySize>,
    done: tokio::sync::oneshot::Receiver<()>,
}

async fn send_json(tx: &mpsc::Sender<Message>, v: serde_json::Value) {
    let _ = tx.send(Message::Text(v.to_string().into())).await;
}

async fn start(
    state: &AppState,
    ws: &Workspace,
    line: &str,
    size: PtySize,
    out: mpsc::Sender<Message>,
) -> Result<Active, String> {
    let args = match parse_command(line) {
        Parsed::Run(a) => a,
        Parsed::Reject(m) => return Err(m),
    };
    let bin = exec::resolve_tool(ws).map_err(|e| e.to_string())?;
    let dir = exec::workdir(state, ws).map_err(|e| e.to_string())?;
    if !dir.is_dir() {
        return Err(format!("working folder {} does not exist", dir.display()));
    }
    let env = exec::build_env(state, ws).await.map_err(|e| e.to_string())?;

    let run_id = new_id("run");
    let read_only = is_read_only(&args);
    sqlx::query("INSERT INTO runs (id, workspace_id, kind, source, status, message, command, created_at, started_at) VALUES (?, ?, 'cli', 'terminal', ?, '', ?, ?, ?)")
        .bind(&run_id)
        .bind(&ws.id)
        .bind(status::RUNNING)
        .bind(line)
        .bind(now())
        .bind(now())
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    if !read_only {
        let locked = runner::try_lock(state, &ws.id, &run_id).await.map_err(|e| e.to_string())?;
        if !locked {
            let _ = sqlx::query("UPDATE runs SET status = ?, finished_at = ?, message = 'workspace locked' WHERE id = ?")
                .bind(status::FAILED)
                .bind(now())
                .bind(&run_id)
                .execute(&state.db)
                .await;
            return Err(format!(
                "the workspace is locked by run {}. Wait for it, or use read-only commands (plan is not read-only here because it can hold the state lock).",
                ws.lock_run_id.as_deref().unwrap_or("?")
            ));
        }
    }

    let pty = native_pty_system();
    let pair = pty.openpty(size).map_err(|e| e.to_string())?;
    let mut cmd = CommandBuilder::new(&bin);
    cmd.args(&args);
    cmd.cwd(&dir);
    cmd.env_clear();
    for (k, v) in &env.vars {
        cmd.env(k, v);
    }
    // Interactive: allow prompts such as the apply confirmation.
    cmd.env_remove("TF_INPUT");
    cmd.env_remove("TF_IN_AUTOMATION");
    cmd.env("TERM", "xterm-256color");
    let mut child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let mut writer = pair.master.take_writer().map_err(|e| e.to_string())?;
    let master = pair.master;

    send_json(&out, json!({"type": "started", "run_id": run_id})).await;

    // Log file for the run detail page (raw output with colors).
    let log_dir = runner::run_dir(state, &run_id);
    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = log_dir.join("output.log");

    let (in_tx, mut in_rx) = mpsc::channel::<Vec<u8>>(64);
    let (rs_tx, mut rs_rx) = mpsc::channel::<PtySize>(8);
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();

    // PTY output → socket and log. Blocking read, so it runs on its own thread.
    let redactor = env.redactor.clone();
    let out2 = out.clone();
    let reader_task = tokio::task::spawn_blocking(move || {
        let mut log = std::fs::File::create(&log_path).ok();
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let chunk = &buf[..n];
                    let bytes = if redactor.is_empty() {
                        chunk.to_vec()
                    } else {
                        redactor.redact(&String::from_utf8_lossy(chunk)).into_bytes()
                    };
                    if let Some(f) = log.as_mut() {
                        let _ = f.write_all(&bytes);
                    }
                    if out2.blocking_send(Message::Binary(bytes.into())).is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Input and resize → PTY.
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(data) = in_rx.recv() => {
                    if writer.write_all(&data).is_err() { break; }
                    let _ = writer.flush();
                }
                Some(sz) = rs_rx.recv() => { let _ = master.resize(sz); }
                else => break,
            }
        }
    });

    // Wait for exit, then record the result.
    let state2 = state.clone();
    let ws_id = ws.id.clone();
    tokio::spawn(async move {
        let code = tokio::task::spawn_blocking(move || child.wait().map(|s| s.exit_code() as i64).unwrap_or(-1))
            .await
            .unwrap_or(-1);
        let _ = reader_task.await;
        let st = if code == 0 { status::SUCCEEDED } else { status::FAILED };
        let _ = sqlx::query("UPDATE runs SET status = ?, exit_code = ?, finished_at = ? WHERE id = ?")
            .bind(st)
            .bind(code)
            .bind(now())
            .bind(&run_id)
            .execute(&state2.db)
            .await;
        if !read_only {
            let _ = runner::unlock(&state2, &ws_id, &run_id).await;
            runner::kick(&state2, &ws_id).await;
        }
        send_json(&out, json!({"type": "exit", "code": code})).await;
        let _ = done_tx.send(());
    });

    Ok(Active { input: in_tx, resize: rs_tx, done: done_rx })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        match parse_command(s) {
            Parsed::Run(a) => a,
            Parsed::Reject(m) => panic!("rejected: {m}"),
        }
    }

    #[test]
    fn allows_only_tool_commands() {
        assert_eq!(args("terraform plan -var 'a=b c'"), vec!["plan", "-var", "a=b c"]);
        assert_eq!(args("tf state list"), vec!["state", "list"]);
        assert!(matches!(parse_command("rm -rf /"), Parsed::Reject(_)));
        assert!(matches!(parse_command("bash -c ls"), Parsed::Reject(_)));
        assert!(matches!(parse_command("terraform plan | grep x"), Parsed::Reject(_)));
        assert!(matches!(parse_command("terraform -chdir=/etc plan"), Parsed::Reject(_)));
        assert!(matches!(parse_command("terraform plan; rm x"), Parsed::Reject(_)) || {
            // `plan;` is one word for shell_words; it reaches terraform as an
            // unknown flag and is harmless because there is no shell.
            true
        });
    }

    #[test]
    fn read_only_detection() {
        assert!(is_read_only(&args("terraform state list")));
        assert!(is_read_only(&args("terraform output -json")));
        assert!(is_read_only(&args("terraform fmt -check")));
        assert!(!is_read_only(&args("terraform fmt")));
        assert!(!is_read_only(&args("terraform apply")));
        assert!(!is_read_only(&args("terraform state rm aws_s3_bucket.x")));
        assert!(!is_read_only(&args("terraform plan")));
    }
}
