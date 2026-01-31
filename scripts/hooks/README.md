# Hook Scripts

Hook scripts for integrating Claude Code with the Agent Console Dashboard. These
scripts are invoked by Claude Code at session lifecycle events and update the
dashboard via the `acd` CLI.

For full setup instructions including settings.json configuration, see
[docs/integration/claude-code-hooks.md](../../docs/integration/claude-code-hooks.md).

## Scripts

### stop.sh

Runs when a Claude Code session stops or completes. Sets the session status to
"attention" so the dashboard highlights sessions that have finished and may need
user review.

- **Event name:** `Stop`
- **Status set:** `attention`

### user-prompt-submit.sh

Runs when a user submits a prompt to Claude Code. Sets the session status to
"working" so the dashboard shows which sessions are actively processing.

- **Event name:** `UserPromptSubmit`
- **Status set:** `working`

### notification.sh

Runs when Claude Code sends a notification (question, error, permission request,
etc.). Sets the session status to "attention" and optionally triggers an
OS-level notification (macOS/Linux lines included but commented out).

- **Event name:** `Notification`
- **Status set:** `attention`

## How Hooks Work

Claude Code invokes registered hook scripts at specific lifecycle events,
passing JSON data via stdin. Each JSON payload includes a `session_id` field
that uniquely identifies the session. The hook scripts parse this JSON using
`jq` to extract `session_id` and call the `acd` CLI to update the dashboard.

If the dashboard daemon is not running, hooks fail gracefully (exit 0) and do
not block Claude Code.

### JSON Stdin Fields

Every hook receives at minimum:

| Field             | Type   | Description                     |
| ----------------- | ------ | ------------------------------- |
| `session_id`      | string | Unique session identifier       |
| `cwd`             | string | Current working directory       |
| `transcript_path` | string | Path to conversation transcript |
| `permission_mode` | string | Current permission mode         |
| `hook_event_name` | string | Which hook fired (e.g., `Stop`) |

## Installation

```sh
mkdir -p ~/.claude/hooks
cp scripts/hooks/*.sh ~/.claude/hooks/
chmod +x ~/.claude/hooks/*.sh
```

Then register the hooks in `~/.claude/settings.json`:

```json
{
  "hooks": {
    "Stop": ["~/.claude/hooks/stop.sh"],
    "UserPromptSubmit": ["~/.claude/hooks/user-prompt-submit.sh"],
    "Notification": ["~/.claude/hooks/notification.sh"]
  }
}
```

See the [full setup guide](../../docs/integration/claude-code-hooks.md) for
detailed instructions, merging with existing settings, and troubleshooting.

## Requirements

- `jq` must be installed and available in PATH (used to parse JSON from stdin)
- `acd` binary must be installed and available in PATH (the CLI for the
  dashboard daemon)
- Claude Code v2.0.76+ for full hook support
