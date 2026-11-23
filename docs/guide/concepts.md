# Concepts

Five ideas explain almost everything in Tuffie.

## Workspace

A workspace is one Terraform or OpenTofu configuration: a folder with `.tf` files, plus its variables, runs and settings. The folder is on your disk, or it is a Git repository that Tuffie clones.

The workspace uses the backend that your configuration defines (`local`, `s3`, …). Tuffie does not move or copy your state.

## Run

A run is one execution of Terraform. There are five kinds:

| Kind | What it does | Applies? |
|---|---|---|
| Plan and apply | `plan`, then `apply` after you confirm | Yes, after confirmation |
| Plan only | `plan` | Never |
| Check for drift | `plan -refresh-only` | Never |
| Refresh state | `plan -refresh-only`, then apply | Yes, after confirmation. Changes state only. |
| Destroy | `plan -destroy`, then apply | Yes, after confirmation |

Commands that you type in the terminal are also saved as runs, with the kind **Terminal**.

## Lock and queue

Only one run at a time changes a workspace. The run that works holds the **workspace lock**. Other runs wait in a queue, in order.

A plan that waits for your confirmation **keeps the lock**. So nothing can change the state between the plan you read and the apply. Confirm or discard the plan to release the lock.

## Variables and precedence

Values come from several places. The last one wins:

1. The environment of the Tuffie process (local mode only).
2. Global variable sets.
3. Variable sets assigned to the workspace.
4. The workspace's own variables.

Every run records where each value came from. See [Variables](variables.md).

## Policy check

After each plan, Tuffie can check the plan against rules written in Rego. A **mandatory** rule that fails blocks the apply. An **advisory** rule only warns. See [Policies](policies.md).

## How a run flows

```
Start run → queue → init → plan → policy check → wait for confirmation → apply → state snapshot
```
