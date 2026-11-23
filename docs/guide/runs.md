# Runs and plan review

## Start a run

1. Open a workspace.
2. Click **Start run**.
3. Select the kind:
   - **Plan and apply** — the normal change workflow.
   - **Plan only** — see what would change. It never applies.
   - **Check for drift** — see what changed outside Terraform. See [Drift detection](drift.md).
   - **Refresh state** — accept drift: update the state to match the real infrastructure.
   - **Destroy infrastructure** — plan the deletion of every resource.

Tuffie opens the run page. The log streams live.

## The run page

The run page shows the run as a timeline:

1. **Plan** — the result of `init` and `plan`. Two tabs:
   - **Changes:** every resource that changes. The symbol shows the action: `+` create, `~` update, `±` replace, `−` destroy, `≤` read.
   - **Log:** the full output.
2. **Policy check** — shown when policy sets apply. See [Policies](policies.md).
3. **Apply** — the confirmation panel, then the apply log.

Below the timeline, **Variables** shows every variable the run used and where it came from.

### Read the changes

- Use the filter buttons (**create**, **update**, **delete** …) to show one kind of change.
- Type in **Filter by address** to find a resource.
- Click a resource to see its attributes.
  - A value with a line through it is the old value. The value after `→` is the new one.
  - *(known after apply)* means Terraform learns the value during apply.
  - **sensitive** means Terraform marks the value as secret. Tuffie never shows it.
- If the plan found changes made outside Terraform, a yellow bar shows the drift count.

## Confirm or discard

When a plan has changes, the run waits with **Needs confirmation**.

1. Read the changes.
2. Optional: write a comment. It is saved with the run.
3. Click **Confirm and apply**. Or click **Discard run** to stop.

> **Note:** While the run waits, it holds the workspace lock. Other runs wait in the queue. Confirm or discard to let them start.

Tuffie applies **exactly** the plan you read. It uses the saved plan file, not a new plan.

## Cancel a run

Click **Cancel run** while a run is queued, planning or applying. Tuffie sends an interrupt, like pressing Ctrl+C. Terraform then stops at a safe point and releases its state lock.

## Run status

| Status | Meaning |
|---|---|
| Pending | Waiting in the queue. |
| Planning / Applying | Terraform is working. |
| Needs confirmation | The plan has changes. It waits for you. |
| Planned | The plan finished. Nothing more happens (no changes, or plan only). |
| Applied | The apply finished. |
| Policy failed | A mandatory policy failed. The plan cannot be applied. |
| Discarded / Canceled | Stopped by a person. |
| Errored | Terraform failed. Read the **Errors** box and the log. |
| Drift detected / No drift | Result of a drift check. |

## Run history

The **Runs** tab lists every run. Use the buttons to show **All**, **UI runs** or **Terminal** commands. Scheduled drift checks are marked *(scheduled)*.
