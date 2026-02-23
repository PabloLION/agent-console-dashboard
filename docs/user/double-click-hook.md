# Double-Click Hook

The TUI supports configurable hooks that fire when a session is double-clicked
in the dashboard. Multiple hooks can be defined per event — they run
sequentially. Each hook receives full session context as environment variables
and as JSON on stdin.

## Configuration

Configure hooks in your TOML config file at
`~/.config/agent-console-dashboard/config.toml` using the
`[[tui.activate_hooks]]` and `[[tui.reopen_hooks]]` array-of-tables syntax
(double brackets):

```toml
# Fires when double-clicking a non-closed session
[[tui.activate_hooks]]
command = 'zellij action go-to-tab-name "$(basename "$ACD_WORKING_DIR")" --session "$ZELLIJ_SESSION_NAME"'
timeout = 5

# Second activate hook runs after the first
[[tui.activate_hooks]]
command = 'echo "activated $ACD_SESSION_ID at $ACD_WORKING_DIR" >> /tmp/acd-hooks.log'
timeout = 2

# Fires when double-clicking a closed session
[[tui.reopen_hooks]]
command = 'zellij action new-tab --name "$(basename "$ACD_WORKING_DIR")" --cwd "$ACD_WORKING_DIR" --session "$ZELLIJ_SESSION_NAME"'
timeout = 5
```

### Hook Object Fields

Each hook entry has two fields:

- `command` — shell command passed to `sh -c` (required)
- `timeout` — seconds to wait before killing the process (optional, default `5`)

The hook process is killed if it exceeds `timeout`. Execution continues to the
next hook regardless of whether the previous hook succeeded or was killed.

## Environment Variables

The hook process receives these variables set in its environment:

- `ACD_SESSION_ID` — the session's unique identifier
- `ACD_WORKING_DIR` — the working directory path (empty string if unknown)
- `ACD_STATUS` — current status (`working`, `attention`, `question`, `closed`)

Use these directly in your hook command:

```toml
[[tui.activate_hooks]]
command = 'code "$ACD_WORKING_DIR"'
timeout = 5
```

## JSON Payload (stdin)

Each hook also receives a JSON payload on stdin containing the full session
snapshot. This follows the same pattern as Claude Code hooks and is useful for
advanced hooks that need fields not available as environment variables.

### SessionSnapshot Schema

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "agent_type": "claudecode",
  "status": "working",
  "working_dir": "/home/user/project",
  "elapsed_seconds": 3600,
  "idle_seconds": 120,
  "closed": false,
  "history": [
    {
      "status": "working",
      "at_secs": 1707900000
    },
    {
      "status": "attention",
      "at_secs": 1707903600
    }
  ]
}
```

### Field Descriptions

| Field             | Type             | Description                                     |
| ----------------- | ---------------- | ----------------------------------------------- |
| `session_id`      | string           | Unique session identifier                       |
| `agent_type`      | string           | Agent type (e.g., `"claudecode"`)               |
| `status`          | string           | Current status (see Status Values below)        |
| `working_dir`     | string or null   | Working directory path, or `null` if unknown    |
| `elapsed_seconds` | integer          | Seconds since session entered current status    |
| `idle_seconds`    | integer          | Seconds since last hook activity                |
| `closed`          | boolean          | Whether the session has been closed             |
| `history`         | array of objects | Status change history (see History Entry below) |

### Status Values

Valid status strings:

- `"working"` — agent is actively working
- `"attention"` — agent needs attention (error/warning)
- `"question"` — agent is asking a question
- `"closed"` — session has been closed

### History Entry

Each history entry records a status change:

| Field     | Type    | Description                           |
| --------- | ------- | ------------------------------------- |
| `status`  | string  | The new status after this transition  |
| `at_secs` | integer | Unix timestamp when this status began |

History is a bounded queue with approximately 10 entries, sorted oldest to
newest.

## TOML Syntax Notes

The `[[tui.activate_hooks]]` double-bracket syntax creates an array of objects.
Each `[[...]]` block appends one entry to the array.

For commands with double quotes, use TOML single-quoted strings — no escaping
needed:

```toml
[[tui.activate_hooks]]
command = 'zellij action go-to-tab-name "$(basename "$ACD_WORKING_DIR")"'
timeout = 5
```

Alternatively, escape with backslashes in double-quoted strings:

```toml
[[tui.activate_hooks]]
command = "zellij action go-to-tab-name \"$(basename \"$ACD_WORKING_DIR\")\""
timeout = 5
```

Single-quoted strings (no escaping) are cleaner for complex shell commands.

## Example Hooks

### Zellij — focus the matching tab

```toml
[[tui.activate_hooks]]
command = 'zellij action go-to-tab-name "$(basename "$ACD_WORKING_DIR")" --session "$ZELLIJ_SESSION_NAME"'
timeout = 5
```

### Zellij — open a new tab for a closed session

```toml
[[tui.reopen_hooks]]
command = 'zellij action new-tab --name "$(basename "$ACD_WORKING_DIR")" --cwd "$ACD_WORKING_DIR" --session "$ZELLIJ_SESSION_NAME"'
timeout = 5
```

### VS Code — open the folder

```toml
[[tui.activate_hooks]]
command = 'code "$ACD_WORKING_DIR"'
timeout = 10
```

### Log to file

```toml
[[tui.activate_hooks]]
command = 'echo "activated $ACD_SESSION_ID at $ACD_WORKING_DIR (status: $ACD_STATUS)" >> /tmp/acd-hooks.log'
timeout = 2
```

### Read JSON with `jq`

```toml
[[tui.activate_hooks]]
command = "jq -r '.session_id' | pbcopy"
timeout = 5
```

This copies the session ID to the clipboard (reads from stdin JSON).

### Multiple hooks in sequence

```toml
[[tui.activate_hooks]]
command = 'zellij action go-to-tab-name "$(basename "$ACD_WORKING_DIR")" --session "$ZELLIJ_SESSION_NAME"'
timeout = 5

[[tui.activate_hooks]]
command = 'echo "$(date): activated $ACD_SESSION_ID" >> /tmp/acd-hooks.log'
timeout = 2
```

### Rust Hook

Rust programs can deserialize the JSON from stdin using the
`agent-console-dashboard` crate:

```rust
use agent_console_dashboard::SessionSnapshot;
use std::io::{self, Read};

fn main() -> anyhow::Result<()> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    let snapshot: SessionSnapshot = serde_json::from_str(&buffer)?;
    println!("Session: {}", snapshot.session_id);
    println!("Status: {}", snapshot.status);
    Ok(())
}
```

### Shell Script

```bash
#!/bin/bash
# Environment variables are set directly — no stdin parsing needed for simple cases
echo "Activated: $ACD_SESSION_ID at $ACD_WORKING_DIR (status: $ACD_STATUS)" >> /tmp/hook.log

# For advanced use, read the full JSON from stdin
json=$(cat)
history_count=$(echo "$json" | jq '.history | length')
echo "History entries: $history_count" >> /tmp/hook.log
```

## No Hook Configured

If no hooks are configured and you double-click a session, the TUI displays a
status message showing the config file path where you can add
`[[tui.activate_hooks]]` or `[[tui.reopen_hooks]]` entries.
