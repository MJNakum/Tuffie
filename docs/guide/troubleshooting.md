# Troubleshooting

## "`terraform` was not found on PATH"

Tuffie cannot find the tool.

1. Run `which terraform` (or `which tofu`) in your shell.
2. If it prints nothing, install the tool.
3. If Tuffie runs from a different environment (for example a service), set the **Binary path** in the workspace **Settings**.

## The page says "host not allowed"

You opened Tuffie with a host name that it does not know, for example through a reverse proxy. Start Tuffie with `--allowed-host your.host.name`.

## "cross-origin request blocked"

A page on another site tried to call Tuffie. This is the protection working. If you see it from your own proxy, make sure the proxy keeps the original `Host` header.

## A run waits in "Pending"

Another run holds the workspace lock. Open the run that the **Locked** badge links to.

- If it says **Needs confirmation**, confirm or discard it.
- If it is still working, wait, or cancel it.

## A terminal command is refused because of the lock

A UI run holds the lock. Read-only commands (`state list`, `show`, `output` …) still work. Wait for the run, or cancel it.

## "Tuffie stopped while this run was active"

Tuffie stopped during a run. The run is marked **Errored**, and its lock is released. Terraform may still hold its own state lock in your backend. If the next plan says the state is locked, run `tf force-unlock <LOCK_ID>` in the terminal, but only when you are sure nothing else is running.

## A policy shows "error"

- **Syntax error:** open the policy and save it. The compile error appears above the editor.
- **Folder source:** check that the folder exists and that Tuffie can read it.
- **Timeout:** a policy took longer than 10 seconds. Make the rule simpler.

## Variables or plans cannot be read ("decryption failed")

The master key changed. Restore the original `master.key`, or set the original `TUFFIE_MASTER_KEY`.

## Reset the guided tour

Click **Help → Guided tour**. To show the welcome tour again on start, click **Help → Reset tours**.

## Get more detail

Start Tuffie with `TUFFIE_LOG=debug` and read the terminal output.
