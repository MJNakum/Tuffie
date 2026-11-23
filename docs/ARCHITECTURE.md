# Tuffie architecture

This document explains how Tuffie works inside. Read it before you change the run engine, the terminal or the security checks.

## Overview

Tuffie is one process. It serves a REST + WebSocket API and the web UI. It does not reimplement Terraform: it runs the real `terraform` or `tofu` binary and reads its machine-readable output.

```mermaid
flowchart LR
  subgraph Browser
    UI[SvelteKit SPA]
    XT[xterm.js terminal]
  end
  subgraph Tuffie["tuffie (one Rust binary)"]
    G[Security guard<br/>Host + Origin]
    API[REST API<br/>axum]
    WS[WebSockets<br/>run stream, terminal]
    R[Run engine<br/>queue + lock]
    T[Terminal<br/>PTY, no shell]
    C[Crypto<br/>XChaCha20-Poly1305]
    DB[(SQLite)]
    FS[(Data folder<br/>runs, checkouts)]
  end
  TF[terraform / tofu]
  UI --> G --> API
  XT --> G --> WS
  API --> R --> TF
  WS --> T --> TF
  R --> DB
  R --> FS
  API --> C --> DB
```

## Source layout

| Path | What it holds |
|---|---|
| `src/main.rs` | Start-up: config, database, master key, recovery, HTTP server. |
| `src/config.rs` | Command-line flags and `TUFFIE_*` environment variables. |
| `src/api/` | REST handlers: workspaces, runs, variables, files, state. |
| `src/runner.rs` | Run engine: queue, lock, plan → confirm → apply, event log. |
| `src/terminal.rs` | Terminal WebSocket: command parsing, PTY, run recording. |
| `src/exec.rs` | Tool lookup, working folder, environment, secret redaction. |
| `src/crypto.rs` | Encryption at rest. |
| `src/security.rs` | Host and Origin checks, security headers. |
| `src/git.rs` | Clone and sync Git sources with the system `git`. |
| `src/assets.rs` | Serves the embedded UI (`web/build`). |
| `migrations/` | SQLite schema. |
| `web/` | SvelteKit 3 single-page app. |
| `tests/fixtures/mock-terraform` | Fake `terraform` used by tests. |

## Run lifecycle

```mermaid
stateDiagram-v2
  [*] --> pending: Start run
  pending --> planning: lock taken
  pending --> canceled: cancel
  planning --> planned_and_finished: no changes, or plan only
  planning --> planned: changes found
  planning --> errored
  planning --> canceled: cancel (SIGINT)
  planned --> applying: confirm (or auto-apply)
  planned --> discarded: discard
  applying --> applied
  applying --> errored
  planned_and_finished --> [*]
  applied --> [*]
  discarded --> [*]
  errored --> [*]
  canceled --> [*]
```

Steps of one run:

1. The API inserts a `pending` run and calls `kick()` for the workspace.
2. `kick()` takes the oldest pending run. It takes the workspace lock with one atomic `UPDATE … WHERE lock_run_id IS NULL`. If another run holds the lock, the run waits.
3. `init -input=false` runs every time. A shared plugin cache (`TF_PLUGIN_CACHE_DIR`) keeps it fast.
4. `plan -json -detailed-exitcode -out=plan.tfplan` runs. Exit code `2` means changes. Tuffie reads each JSON line and writes it as a run event. `change_summary` sets the add/change/destroy counts.
5. `show -json plan.tfplan` exports the structured plan. Tuffie encrypts it to `plan.json.enc`.
6. With changes, the run becomes `planned` and **keeps the lock**. Nothing else can change the state before the apply.
7. On confirm, `apply -json plan.tfplan` runs the exact plan that was reviewed.
8. After apply, `state pull` saves an encrypted state snapshot. This gives state history with any backend.
9. The lock is released and `kick()` starts the next pending run.

On start, runs that were active when Tuffie stopped are marked `errored`, and their locks are released.

### Run events

Each run has `runs/<id>/events.jsonl`. A line is:

```json
{"seq": 12, "ts": "…", "stage": "plan", "level": "info", "message": "Plan: 2 to add…", "type": "change_summary", "data": {…}}
```

The `/api/runs/{id}/stream` WebSocket subscribes to the live channel first, then sends the file, then the live events. It drops duplicates by `seq`, so no event is lost or repeated.

## Terminal

The terminal is the most sensitive feature, because it runs programs.

- The browser runs the line editor (history, Tab completion, cursor keys). The server receives **one complete command line**.
- The server splits the line like a shell (`shell-words`), but **there is no shell**. The first word must be `terraform`, `tofu` or `tf`. The server runs the workspace's binary directly in a PTY.
- Pipe and redirect tokens (`|`, `;`, `&&`, `>`) and `-chdir` are refused.
- The working folder is always the workspace folder.
- Workspace variables are set. `TF_INPUT` is not set, so interactive prompts (for example "Enter a value: yes") work.
- Every command is saved as a run with `kind = cli` and `source = terminal`. The raw output (with colors) is saved to `runs/<id>/output.log`, with secrets redacted.
- Commands that can change state or files take the workspace lock. Read-only commands (`state list`, `show`, `output`, `validate`, `fmt -check`, …) do not.

Protocol: see the comment at the top of `src/terminal.rs`.

## Security model

### Threats and defences

| Threat | Defence |
|---|---|
| Another web site calls the local API (CSRF) | `Origin` must match `Host` for writes and WebSocket upgrades. |
| DNS rebinding to `127.0.0.1` | `Host` must be a loopback name or an `--allowed-host`. |
| Local mode exposed on the network | Local mode refuses non-loopback `--listen` addresses. |
| Secrets read from the database or disk | XChaCha20-Poly1305 for variables, plan JSON and state snapshots. The record or run id is the associated data, so a copied ciphertext does not decrypt in another place. |
| Secrets shown in the UI | Sensitive variables are write-only. Values that Terraform marks as sensitive are masked in plan and state views. |
| Secrets in logs | Sensitive variable values are replaced with `***` in run events and terminal output. |
| Leftover plan files | `plan.tfplan` is deleted after apply, discard, or a plan with no changes. |
| Path traversal in the file editor | Relative paths only, no `..`, canonical path must stay inside the source root (stops symlink escapes). `.terraform`, `.git` and `.tfstate` files are blocked. |
| Command injection from the terminal | No shell. Allow-list for the program name. `-chdir` refused. |
| Malicious Git URL | Only `https://`, `ssh://`, `git@`, `file://`. `protocol.ext.allow=never`, hooks disabled, no prompts. |
| Server mode without login | Refused unless `--unsafe-no-auth` is given (for testing behind your own auth proxy). |

### Known limits (v0.1)

- In local mode, runs see your shell environment (`--inherit-env`). This is on purpose: Tuffie must work like your terminal. In server mode it is off.
- A Terraform configuration can run any code (providers, `local-exec`, `external` data sources). Tuffie cannot make untrusted configurations safe. Phase 2 adds container isolation for each run.
- Redaction replaces exact values only. A secret that Terraform changes (for example base64) is not redacted. Values that Terraform marks as sensitive are still masked in plans and state.
- The terminal limits commands, but `terraform console` can still read files through `file()`. Every configuration can do this too.

## Data folder

```
<data-dir>/            mode 0700
  tuffie.db            SQLite (WAL)
  master.key           base64 key, mode 0600 (not used when TUFFIE_MASTER_KEY is set)
  plugin-cache/        shared provider cache
  checkouts/<ws-id>/   Git sources
  cache/<ws-id>.count  resource count for the workspace list
  runs/<run-id>/
    events.jsonl       run events
    plan.tfplan        binary plan, only while it can be applied
    plan.json.enc      encrypted structured plan
    state.json.enc     encrypted state snapshot after apply
    output.log         terminal output (CLI runs)
```

## API

All paths start with `/api`.

| Method | Path | Purpose |
|---|---|---|
| GET | `/system/info` | Version, mode, detected tools. |
| GET, POST | `/workspaces` | List, create. |
| GET, PATCH, DELETE | `/workspaces/{id}` | Read, update, delete. |
| POST | `/workspaces/{id}/sync` | Fetch and check out the Git ref. |
| GET, POST | `/workspaces/{id}/runs` | List, start (`kind`: `plan_only`, `plan_and_apply`, `destroy`, `refresh`). |
| GET | `/runs/{id}` | Run details. |
| GET | `/runs/{id}/events` | All events. |
| WS | `/runs/{id}/stream` | Live events. |
| GET | `/runs/{id}/plan` | Structured plan, sensitive values masked. |
| GET | `/runs/{id}/output` | Terminal output of a CLI run. |
| GET | `/runs/{id}/state` | State snapshot saved after this run. |
| POST | `/runs/{id}/apply` | Confirm. Body: `{"comment": "…"}`. |
| POST | `/runs/{id}/discard` | Discard a planned run. |
| POST | `/runs/{id}/cancel` | Cancel a pending or active run. |
| GET, POST | `/workspaces/{id}/variables` | List, create. |
| PATCH, DELETE | `/variables/{id}` | Update, delete. |
| GET | `/workspaces/{id}/files` | File tree. |
| GET, PUT | `/workspaces/{id}/files/content?path=` | Read, write one file. |
| GET | `/workspaces/{id}/state` | Current state (`show -json`), masked. |
| GET | `/workspaces/{id}/state-versions` | Saved snapshots. |
| WS | `/workspaces/{id}/terminal` | Terminal. |

## Design system

The UI uses one accent colour (signal blue) on graphite surfaces. The tokens are in `web/src/app.css`.

| Token | Dark | Light | Use |
|---|---|---|---|
| `--bg` | `#08090c` | `#f6f7f9` | Page background |
| `--surface` | `#0e1015` | `#ffffff` | Cards, panels |
| `--border` | `#20252f` | `#e1e5ec` | Dividers |
| `--fg` | `#e8ebf1` | `#0d1117` | Main text |
| `--primary` | `#3d7eff` | `#1f5fff` | Actions, focus, selection only |
| `--success` | `#2fbf71` | `#138a4b` | Create, applied |
| `--warning` | `#e5a50a` | `#a76f00` | Update, needs confirmation |
| `--danger` | `#f0524f` | `#d22d2a` | Destroy, errors |
| `--violet` | `#a585ff` | `#6d48e0` | Replace, data sources |

Rules:

- Do not use pure black. Near-black with a cool tint reduces eye strain in long sessions.
- Show depth with surface tone steps, not with shadows or gradients.
- Use blue only where the user can act. Status colours follow Terraform's plan symbols (`+ ~ − ±`).
- Inter for UI text, JetBrains Mono for code, addresses and IDs. Ligatures are off in code.
