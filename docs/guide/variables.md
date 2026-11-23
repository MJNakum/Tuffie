# Variables and variable sets

## Kinds of variables

| Kind | Set as | Use for |
|---|---|---|
| Terraform variable | `TF_VAR_<key>` | Values for `variable` blocks |
| Environment variable | `<key>` | Provider settings, for example `AWS_PROFILE` or `ARM_CLIENT_ID` |

## Add a workspace variable

1. Open the workspace, then the **Variables** tab.
2. Click **Add variable** in the correct section.
3. Enter the **key** and the **value**.
4. Optional settings:
   - **HCL:** Terraform parses the value as HCL. Use it for lists and maps, for example `["10.0.0.0/16", "10.1.0.0/16"]`.
   - **Sensitive:** the value becomes write-only. See below.
5. Click **Add variable**.

## Sensitive values

- Tuffie never shows a sensitive value again, in the UI or in the API.
- Tuffie replaces the value with `***` in run logs and terminal output.
- To change it, edit the variable and enter a new value. An empty field keeps the old value.
- A sensitive variable cannot become non-sensitive. Delete it and create it again.

> **Note:** Tuffie encrypts **every** variable value at rest, sensitive or not.

## Variable sets

A variable set is a group of variables that many workspaces use. Typical sets: cloud credentials for one account, common tags, a default region.

### Create a variable set

1. Click **Variable sets** in the sidebar.
2. Click **New variable set**.
3. Enter a name and a description.
4. Select the scope:
   - **All workspaces:** applies to every workspace, also future ones.
   - **Selected workspaces:** select the workspaces in the list.
5. Click **Create variable set**.
6. Add variables to the set. They work the same as workspace variables.

### Precedence

When two sources set the same key, the last one wins:

1. Global sets, sorted by name.
2. Assigned sets, sorted by name.
3. The workspace's own variables.

**Example:** the global set `org-defaults` sets `aws_region = us-east-1`. The workspace sets `aws_region = eu-west-1`. The run uses `eu-west-1`.

The workspace **Variables** tab shows **Inherited from variable sets**. Overridden keys have a line through them.

## Where did a value come from?

Every run records its variables. On the run page, open **Variables** below the timeline. For each key you see the source (workspace or set) and whether a later source overrode it. Tuffie records only keys and sources, never values.
