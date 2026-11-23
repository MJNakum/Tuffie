# Terminal

The terminal runs `terraform` and `tofu` commands inside Tuffie, in the workspace folder.

## Open and close

- Press `` Ctrl+` `` or click **Terminal** in the workspace header.
- Drag the top edge of the panel to change its height.
- Click **−** to hide the panel. The session stays open.
- Click **×** to close the session.

The terminal stays open when you change tabs in the same workspace.

## Commands

| You type | Tuffie runs |
|---|---|
| `terraform …` | The workspace's tool |
| `tofu …` | The workspace's tool |
| `tf …` | The workspace's tool (short alias) |
| `help` | Shows help |
| `clear` | Clears the screen |
| `history` | Lists your previous commands |

Workspace variables and variable sets are set for every command, the same as for UI runs.

Interactive prompts work. For example, `tf apply` asks *Enter a value:*. Type `yes` and press Enter.

## Keys

| Key | Action |
|---|---|
| ↑ / ↓ | Previous / next command |
| Tab | Complete commands and subcommands |
| Ctrl+C | Stop the running command, or clear the line |
| Ctrl+L | Clear the screen |
| Ctrl+A / Ctrl+E | Start / end of line |
| Ctrl+U / Ctrl+W | Delete the line / the last word |

## What is not allowed

The terminal is **not** a shell. This keeps it safe, also on a shared server.

- Other programs (`ls`, `cat`, `bash` …) are refused.
- Pipes and redirects (`|`, `>`, `;`, `&&`) are refused.
- `-chdir` is refused. Commands always run in the workspace folder.

Use your own terminal for everything else. Tuffie is a companion to the CLI, not a replacement.

## Commands in run history

Every command is saved as a run. Open the **Runs** tab and select **Terminal** to see them. Click one to see its full output with colors. Sensitive values show as `***`.

## Commands and the lock

Commands that only read take no lock: `show`, `output`, `validate`, `state list`, `state show`, `state pull`, `fmt -check`, `providers`, `graph`, `console`, `version`.

Every other command takes the workspace lock while it runs. If a UI run holds the lock, Tuffie refuses the command and says which run holds it.
