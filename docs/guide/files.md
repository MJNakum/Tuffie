# File editor

The **Files** tab edits the files of the workspace.

## Edit a file

1. Open the **Files** tab.
2. Click a file in the tree. `main.tf` opens first if it exists.
3. Edit the text. A yellow dot shows unsaved changes.
4. Press `Ctrl+S`, or click **Save**.

The editor highlights HCL in `.tf`, `.tfvars` and `.hcl` files. It supports search (`Ctrl+F`), multiple selections and code folding.

## Create a file

1. Click the **new file** icon above the tree.
2. Enter a path relative to the workspace, for example `modules/vpc/variables.tf`. The folder must exist.

## Limits

- Files larger than 2 MB, and files that are not UTF-8 text, do not open.
- The editor cannot leave the workspace folder. Paths with `..` and symlinks to other folders are refused.
- `.terraform`, `.git` and `.tfstate` files are hidden. State files hold secrets. Use the [State](state.md) tab instead.

> **Note:** For a Git workspace, edits change the Tuffie checkout only. Tuffie does not commit or push. *Sync now* overwrites the edits.
