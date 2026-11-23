# Configuration

Set options with command-line flags or with environment variables. A flag wins over an environment variable.

```sh
tuffie serve --listen 127.0.0.1:8080 --data-dir ~/tuffie-data
```

`tuffie` with no subcommand is the same as `tuffie serve`, and it opens your browser.

## Options

| Flag | Environment variable | Default | What it does |
|---|---|---|---|
| `--listen` | `TUFFIE_LISTEN` | `127.0.0.1:7777` | Address and port. Local mode accepts only loopback addresses. |
| `--data-dir` | `TUFFIE_DATA_DIR` | see below | Database, run logs, master key. |
| `--mode` | `TUFFIE_MODE` | `local` | `local`: one user, no login. `server`: a team server (needs SSO, which is on the roadmap). |
| `--allowed-host` | `TUFFIE_ALLOWED_HOSTS` | — | Extra host names to accept, comma-separated. Use behind a reverse proxy. |
| `--inherit-env` | `TUFFIE_INHERIT_ENV` | `true` in local mode | Runs see the environment of the Tuffie process. |
| `--no-open` | `TUFFIE_NO_OPEN` | — | Do not open the browser. |
| — | `TUFFIE_MASTER_KEY` | — | Base64 32-byte encryption key. Use it instead of the key file. |
| — | `TUFFIE_LOG` | `info` | Log filter, for example `debug` or `tuffie=debug`. |

## Data folder

| System | Default |
|---|---|
| Linux | `~/.local/share/tuffie` |
| macOS | `~/Library/Application Support/tuffie` |

The folder holds:

- `tuffie.db` — the database.
- `master.key` — the encryption key (mode `0600`).
- `runs/` — run logs, encrypted plans and encrypted state snapshots.
- `checkouts/` — Git workspaces.
- `samples/` — the sample project from the tour.
- `plugin-cache/` — a shared provider cache, so `init` is fast.

## Back up

1. Stop Tuffie.
2. Copy the whole data folder.

> **Caution:** Without `master.key` (or `TUFFIE_MASTER_KEY`), Tuffie cannot read variables, plans or state snapshots. Keep a copy of the key in a safe place, apart from the database.

## Cloud credentials

In local mode, runs see your shell environment. So `AWS_PROFILE`, `~/.aws/credentials`, `gcloud` logins and `az login` work exactly as in your terminal.

To give a workspace its own credentials, add them as [environment variables](variables.md). Mark secrets as **sensitive**. To share them, use a [variable set](variables.md#variable-sets).

## Upgrade

1. Stop Tuffie.
2. Back up the data folder.
3. Replace the binary.
4. Start Tuffie. It upgrades the database automatically.
