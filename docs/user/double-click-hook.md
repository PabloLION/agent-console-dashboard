# Double-Click Hook

The TUI supports a configurable double-click hook that fires when a session is
double-clicked in the dashboard. The hook receives full session context as JSON
on stdin.

## Configuration

Configure the hook in your TOML config file at
`~/.config/agent-console-dashboard/config.toml`:

```toml
[tui]
double_click_hook = "your-command-here {session_id}"
```

## Placeholder Substitution

The hook command supports three placeholders that are expanded before execution:

- `{session_id}` — replaced with the session identifier
- `{working_dir}` — replaced with the working directory path (or `<none>`)
- `{status}` — replaced with current status (`working`, `attention`, `question`,
  `closed`)

Example:

```toml
[tui]
double_click_hook = "zellij action go-to-tab-name \"{session_id}\""
```

## JSON Payload (stdin)

The hook receives a JSON payload on stdin containing the full session snapshot.
This follows the same pattern as Claude Code hooks.

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

| Field             | Type              | Description                                      |
| ----------------- | ----------------- | ------------------------------------------------ |
| `session_id`      | string            | Unique session identifier                        |
| `agent_type`      | string            | Agent type (e.g., `"claudecode"`)                |
| `status`          | string            | Current status (see Status Values below)         |
| `working_dir`     | string or null    | Working directory path, or `null` if unknown     |
| `elapsed_seconds` | integer           | Seconds since session entered current status     |
| `idle_seconds`    | integer           | Seconds since last hook activity                 |
| `closed`          | boolean           | Whether the session has been closed              |
| `history`         | array of objects  | Status change history (see History Entry below)  |

### Status Values

Valid status strings:

- `"working"` — agent is actively working
- `"attention"` — agent needs attention (error/warning)
- `"question"` — agent is asking a question
- `"closed"` — session has been closed

### History Entry

Each history entry records a status change:

| Field     | Type    | Description                               |
| --------- | ------- | ----------------------------------------- |
| `status`  | string  | The new status after this transition      |
| `at_secs` | integer | Unix timestamp when this status began     |

History is a bounded queue with approximately 10 entries, sorted oldest to
newest.

## Example Hooks

### Read JSON with `jq`

```toml
[tui]
double_click_hook = "jq -r '.session_id' | pbcopy"
```

This copies the session ID to the clipboard.

### Rust Hook

Rust programs can deserialize the JSON using the `agent-console-dashboard`
crate:

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
# Read JSON from stdin
json=$(cat)
session_id=$(echo "$json" | jq -r '.session_id')
status=$(echo "$json" | jq -r '.status')
echo "Clicked session: $session_id (status: $status)" >> /tmp/hook.log
```

## No Hook Configured

If no hook is configured and you double-click a session, the TUI displays a
status message showing the config file path where you can add the
`tui.double_click_hook` setting.
