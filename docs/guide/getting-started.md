# Getting started

This page takes you from nothing to your first applied plan in about five minutes.

## Requirements

- Linux or macOS.
- `terraform` or `tofu` on your `PATH`. Tuffie runs the real binary. It does not replace it.
- To build from source: Rust 1.80 or later, and Node.js 22 or later.

## Install

1. Get the source:

   ```sh
   git clone https://github.com/MJNakum/Tuffie.git
   cd Tuffie
   ```

2. Build the web UI and the binary:

   ```sh
   make build
   ```

   The result is one file: `target/release/tuffie`. Copy it anywhere on your `PATH`.

## Start Tuffie

1. Run:

   ```sh
   tuffie
   ```

2. Tuffie opens `http://127.0.0.1:7777` in your browser. If it does not, open that address yourself.

> **Note:** Tuffie listens only on `127.0.0.1` in local mode. Other computers cannot connect. See [Configuration](configuration.md) to change the address.

On the first start, the **guided tour** opens. It shows each part of the screen. You can replay it at any time from **Help → Guided tour**.

## Create your first workspace

A workspace is one Terraform configuration. You have two choices:

- **Try the sample project.** In the tour, or on the empty Workspaces page, click **Create sample project**. It uses only the `random` provider. It needs no cloud account.
- **Use your own folder.**
  1. Click **New workspace**.
  2. Select **Local folder**.
  3. Enter the absolute path to a folder with `.tf` files, for example `/home/you/infra/networking`.
  4. Enter a name, for example `networking-dev`.
  5. Select **Terraform** or **OpenTofu**.
  6. Click **Create workspace**.

## Run your first plan

1. Open the workspace.
2. Click **Start run → Plan and apply**.
3. Tuffie runs `init`, then `plan`. The log streams live.
4. When the plan finishes, review the changes. Click a resource to see each attribute.
5. Click **Confirm and apply**. Optionally, write a comment first.
6. When the apply finishes, open the **State** tab to see your resources.

> **Caution:** *Confirm and apply* changes real infrastructure. Read the plan first, the same way you read `terraform plan` output.

## Open the terminal

1. Press `` Ctrl+` `` or click **Terminal**.
2. Type `tf state list` and press Enter.

The command also appears in the **Runs** tab. See [Terminal](terminal.md).

## Next steps

- Share cloud credentials between workspaces with [variable sets](variables.md#variable-sets).
- Block risky changes with [policies](policies.md).
- Turn on scheduled [drift detection](drift.md).
