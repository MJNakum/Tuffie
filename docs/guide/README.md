# Tuffie user guide

Tuffie is a fast, local-first UI for Terraform and OpenTofu. It runs as one small binary on your laptop or on a server. Use it next to the CLI: Tuffie adds plan review, state browsing, run history, approvals, policies and drift detection.

The same guide is in the app: click **Help → Documentation**, or open `/docs`.

## Start here

1. [Getting started](getting-started.md) — install Tuffie and run your first plan.
2. [Concepts](concepts.md) — workspaces, runs, the lock and variable precedence in five minutes.

## Use Tuffie

- [Workspaces](workspaces.md) — connect a local folder or a Git repository.
- [Runs and plan review](runs.md) — plan, review, approve, apply, cancel.
- [State](state.md) — browse resources, outputs, the dependency graph and state history.
- [Terminal](terminal.md) — run `terraform` and `tofu` commands inside the UI.
- [File editor](files.md) — edit `.tf` files.
- [Variables and variable sets](variables.md) — set values, share them, keep secrets safe.
- [Policies](policies.md) — check every plan with Rego (OPA) rules.
- [Drift detection](drift.md) — find changes made outside Terraform.

## Run Tuffie

- [Configuration](configuration.md) — flags, environment variables, data folder, backups.
- [Security](security.md) — what Tuffie protects and what it cannot.
- [Keyboard shortcuts](shortcuts.md)
- [Troubleshooting](troubleshooting.md)

For developers: [Architecture](../ARCHITECTURE.md) explains the internals and the HTTP API.
