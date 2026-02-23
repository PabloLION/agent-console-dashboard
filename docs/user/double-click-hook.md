# Double-Click Hook

The TUI supports configurable hooks that fire when a session is double-clicked
in the dashboard. The hook receives full session context as environment
variables and as JSON on stdin.

## Configuration

Configure the hooks in your TOML config file at
`~/.config/agent-console-dashboard/config.toml`:

```toml
[tui]
# Hook for non-closed sessions (double-click to activate)
activate_hook = "zellij action go-to-tab-name \"$(basename \"$ACD_WORKING_DIR\")\""

# Hook for closed sessions (double-click to reopen)
reopen_hook = '''zellij action new-tab --name "$(basename "$ACD_WORKING_DIR")" --cwd "$ACD_WORKING_DIR"'''
```

## Environment Variables

The hook process receives these variables set in its environment:

- `ACD_SESSION_ID` — the session's unique identifier
- `ACD_WORKING_DIR` — the working directory path (empty string if unknown)
- `ACD_STATUS` — current status (`working`, `attention`, `question`, `closed`)

Use these directly in your hook command:

```toml
[tui]
activate_hook = "code \"$ACD_WORKING_DIR\""
```

## JSON Payload (stdin)

The hook also receives a JSON payload on stdin containing the full session
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

## TOML Escaping

Hook commands often contain double quotes (e.g., for shell subcommands). TOML
offers two options:

**Option 1 — Backslash escaping** (basic strings):

```toml
activate_hook = "zellij action go-to-tab-name \"$(basename \"$ACD_WORKING_DIR\")\""
```

**Option 2 — Triple-quoted literal strings** (no escaping needed):

```toml
activate_hook = '''zellij action go-to-tab-name "$(basename "$ACD_WORKING_DIR")"'''
```

Triple-quoted literal strings (`'''...'''`) are the cleaner option for complex
commands.

## Example Hooks

### Zellij — focus the matching tab

```toml
[tui]
activate_hook = '''zellij --session $ZELLIJ_SESSION_NAME action go-to-tab-name "$(basename "$ACD_WORKING_DIR")"'''
```

### Zellij — open a new tab for a closed session

```toml
[tui]
reopen_hook = '''zellij --session $ZELLIJ_SESSION_NAME action new-tab --name "$(basename "$ACD_WORKING_DIR")" --cwd "$ACD_WORKING_DIR"'''
```

### VS Code — open the folder

```toml
[tui]
activate_hook = "code \"$ACD_WORKING_DIR\""
```

### Read JSON with `jq`

```toml
[tui]
activate_hook = "jq -r '.session_id' | pbcopy"
```

This copies the session ID to the clipboard (reads from stdin JSON).

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

If no hook is configured and you double-click a session, the TUI displays a
status message showing the config file path where you can add the
`tui.activate_hook` or `tui.reopen_hook` setting.
