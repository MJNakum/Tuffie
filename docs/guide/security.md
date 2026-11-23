# Security

## What Tuffie protects

| Risk | Protection |
|---|---|
| Secrets on disk | Variable values, saved plans and state snapshots are encrypted with XChaCha20-Poly1305. |
| Secrets on screen | Sensitive variables are write-only. Values that Terraform marks as sensitive are masked in plans and state. |
| Secrets in logs | Sensitive variable values are replaced with `***` in run logs and terminal output. |
| Leftover plan files | Binary plan files are deleted after apply, discard, or a plan with no changes. |
| Other web sites calling your local Tuffie | Tuffie checks the `Host` and `Origin` headers of every request. |
| Access from the network | Local mode listens only on loopback (`127.0.0.1`). |
| Commands from the terminal | No shell. Only `terraform` / `tofu`, only in the workspace folder. |
| Reading files outside the workspace | The editor refuses `..`, absolute paths and symlinks that leave the folder. |
| Unsafe Git URLs | Only `https://`, `ssh://`, `git@` and `file://`. The `ext::` transport and Git hooks are disabled. |
| Policies calling the network | The `http.send` builtin is not available. |

## What Tuffie cannot protect

- **A configuration can run any code.** Providers, `local-exec` and the `external` data source run programs on your machine. Run only configurations that you trust.
- **Local mode shares your environment.** Runs can use every credential that your shell can use. This is on purpose: it makes Tuffie work like your terminal.
- **Redaction matches exact values.** If Terraform transforms a secret (for example base64), the transformed text is not redacted. Values that Terraform marks as sensitive are still masked.
- **Your backend holds the real state.** Protect it with your backend's own access control and encryption.

## Good practice

1. Back up `master.key` apart from the database.
2. Mark every secret as **sensitive**.
3. Keep **Auto-apply** off for production workspaces.
4. Use **mandatory** policies for rules that must never break.
5. Do not expose local mode through a tunnel or proxy. For a team, wait for server mode with SSO.
