# Drift detection

**Drift** is a difference between your real infrastructure and the Terraform state. It happens when someone changes a resource in the cloud console, with a script, or with another tool.

Tuffie finds drift with `terraform plan -refresh-only`. A drift check never changes infrastructure or state.

## Check now

Do one of these:

- On the workspace **Overview**, in the **Drift** card, click **Check now**.
- Click **Start run → Check for drift**.

## Check on a schedule

1. Open the workspace, then **Settings**.
2. In **Drift detection**, select an interval: every hour, every 6 hours, every 12 hours, every day or every week.
3. Click **Save changes**.

The first check starts within a minute. Tuffie skips a check while the workspace has a queued, running or waiting run, so checks never block your work.

> **Note:** Tuffie must be running for scheduled checks. If it is stopped, it does the missed check soon after it starts.

## See the result

- The **Workspaces** list shows a yellow **Drifted** badge.
- The **Drift** card on the Overview shows the count, the time of the last check and the time of the next one.
- Click **details** to open the drift run. It shows each drifted resource as a diff: the old value has a line through it, the new value follows `→`.

## Fix drift

You have two choices:

1. **Keep your configuration:** start **Plan and apply**. Terraform changes the real infrastructure back.
2. **Keep the real infrastructure:** update your `.tf` files to match. Or start **Refresh state** to accept the new values into the state only.

After a successful apply, Tuffie marks the workspace as **No drift**.
