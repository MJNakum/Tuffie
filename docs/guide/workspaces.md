# Workspaces

A workspace connects Tuffie to one Terraform configuration.

## Create a workspace

1. Click **New workspace**.
2. Select the **source**:
   - **Local folder:** a folder on this machine. Tuffie runs commands in it and edits files in place.
   - **Git repository:** Tuffie clones the repository into its data folder.
3. Fill in the source fields:

   | Field | Example | Notes |
   |---|---|---|
   | Folder path | `/home/you/infra/networking` | Absolute path. Must exist. |
   | Repository URL | `git@github.com:acme/infra.git` | `https://`, `ssh://`, `git@` or `file://`. |
   | Branch, tag or commit | `main` | Empty means the default branch. |
   | Working directory | `envs/prod` | Optional sub-folder with the configuration. |

4. Enter a **name**: letters, numbers, `-` and `_`.
5. Select the **tool**: Terraform or OpenTofu. Tuffie shows which ones it found on your `PATH`.
6. Optional: set a **binary path** to use a specific version, for example `/opt/terraform-1.9/terraform`.
7. Optional: turn on **Auto-apply**. Successful plans then apply without confirmation.

> **Caution:** Keep Auto-apply off for production. With Auto-apply on, nobody reviews the plan.

8. Click **Create workspace**.

## Git credentials

Tuffie uses your normal Git setup: your SSH agent, or your Git credential helper. Tuffie never asks for or stores Git passwords.

To get new commits, open **Settings → Sync repository** and click **Sync now**.

> **Caution:** *Sync now* overwrites local edits in the Tuffie checkout of the repository.

## The workspace page

| Tab | What you find |
|---|---|
| Overview | Latest run, resource summary, drift status, details and outputs. |
| Runs | Every run, from the UI, the schedule and the terminal. |
| State | Resources, attributes, dependency graph, outputs, state history. |
| Variables | Workspace variables and the variable sets that apply. |
| Files | The file editor. |
| Settings | Name, tool, working directory, auto-apply, drift schedule, delete. |

## Delete a workspace

1. Open **Settings**.
2. In **Danger zone**, click **Delete workspace**.
3. Type the workspace name and confirm.

Tuffie deletes its own runs, logs and variables. It does **not** destroy infrastructure, does not touch your state, and does not delete your local folder. To destroy infrastructure first, start a **Destroy** run.
