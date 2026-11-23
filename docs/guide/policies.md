# Policies

Policies check every plan before anything is applied. You write them in **Rego**, the language of Open Policy Agent (OPA).

Tuffie evaluates Rego itself. You do not install `opa` or `conftest`.

## How it works

1. A run finishes its plan.
2. Tuffie evaluates every policy set that applies to the workspace. The input is the plan JSON from `terraform show -json`.
3. The **Policy check** step on the run page shows the result.
4. If a **mandatory** set fails, the run ends as **Policy failed**. The plan cannot be applied, and the workspace lock is released.

Drift checks skip the policy check.

## Rules

| Rule name | Result |
|---|---|
| `deny` | Failure |
| `violation` | Failure |
| `warn` | Warning. Never blocks. |

A rule is a set of messages. A message is a string, or an object with a `msg` field. Other fields show as details.

## Enforcement

| Enforcement | A failing `deny` … |
|---|---|
| Advisory | shows as a warning. Apply is allowed. |
| Mandatory | blocks the apply. |

Start new policies as **advisory**. Make them **mandatory** when you trust them.

## Create a policy set

1. Click **Policies** in the sidebar.
2. Click **New policy set**.
3. Enter a name, for example `security-baseline`.
4. Select **Advisory** or **Mandatory**.
5. Select the source:
   - **Write in Tuffie:** you edit the policies in Tuffie.
   - **Folder:** Tuffie reads all `*.rego` files in a folder, for example a Git checkout of your policy repository. Files that end in `_test.rego` are skipped.
6. Select the scope: all workspaces, or the workspaces you choose.
7. Click **Create policy set**.

## Write a policy

1. Open the policy set.
2. Click the **new file** icon and enter a name, for example `s3-tags.rego`.
3. Tuffie adds a template. Edit it.
4. Press `Ctrl+S`, or click **Save**. Tuffie compiles the policy first. A syntax error appears above the editor.

### Example: require an owner tag

```rego
package terraform.tags

import rego.v1

deny contains msg if {
	some rc in input.resource_changes
	rc.type == "aws_s3_bucket"
	"create" in rc.change.actions
	not rc.change.after.tags.owner
	msg := sprintf("%s must have an owner tag", [rc.address])
}
```

### Example: warn before a destroy

```rego
package terraform.safety

import rego.v1

warn contains msg if {
	some rc in input.resource_changes
	"delete" in rc.change.actions
	msg := sprintf("%s will be destroyed", [rc.address])
}
```

### Example: allow only some instance types

```rego
package terraform.cost

import rego.v1

allowed := {"t3.micro", "t3.small", "t3.medium"}

deny contains msg if {
	some rc in input.resource_changes
	rc.type == "aws_instance"
	t := rc.change.after.instance_type
	not t in allowed
	msg := sprintf("%s uses %s; allowed: %v", [rc.address, t, allowed])
}
```

### Useful input fields

| Field | Contains |
|---|---|
| `input.resource_changes[_].address` | Resource address |
| `input.resource_changes[_].type` | Resource type |
| `input.resource_changes[_].change.actions` | `["create"]`, `["update"]`, `["delete"]`, `["delete", "create"]` … |
| `input.resource_changes[_].change.before` | Old values |
| `input.resource_changes[_].change.after` | New values |
| `input.resource_drift` | Changes made outside Terraform |
| `input.configuration` | The configuration (providers, modules, variables) |

## Test a policy

You do not need a new run to test a policy.

1. Open the policy set.
2. In **Test against a plan**, select an earlier run.
3. Click **Run test**.

Tuffie evaluates the set against the saved plan of that run. Nothing is applied.

## Existing Conftest policies

Most Conftest policies work unchanged: Tuffie reads `deny`, `violation` and `warn` in any package. The older Rego syntax (`deny[msg] { … }`) also works.

## Limits

- Policies cannot make network calls. The `http.send` builtin is not available.
- Each evaluation stops after 10 seconds.
