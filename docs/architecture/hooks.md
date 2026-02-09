# Hook Architecture

How ACD integrates with Claude Code via the [hooks system][hooks reference]. For
design rationale, see the [integration design](../design/integrations.md).

## Overview

Agent Console Dashboard registers 6 hooks that Claude Code fires during its
lifecycle. Each hook invokes `acd claude-hook <status>`, which forwards the
status to the ACD daemon over a Unix socket.

| Hook Event                            | Status Set  | When It Fires              |
| ------------------------------------- | ----------- | -------------------------- |
| `SessionStart`                        | `attention` | Session begins or resumes  |
| `UserPromptSubmit`                    | `working`   | User sends a prompt        |
| `Stop`                                | `attention` | Claude finishes responding |
| `SessionEnd`                          | `closed`    | Session ends               |
| `Notification` (`elicitation_dialog`) | `question`  | AskUserQuestion dialog     |
| `Notification` (`permission_prompt`)  | `attention` | Permission prompt          |

## Prerequisites

- ACD installed (`acd` binary in PATH)
- Claude Code with hooks support

## Installation

```sh
# Write all 6 hooks to ~/.claude/settings.json
acd install

# Verify hooks were written
acd status
```

`acd install` writes hook entries to `~/.claude/settings.json` and verifies that
`acd` is in `$PATH`. The command is idempotent (safe to run multiple times).

To remove hooks:

```sh
acd uninstall
```

`acd uninstall` removes only the ACD hook entries from `settings.json`, leaving
other settings intact.

### Plugin path (alternative)

The build system also generates `.claude-plugin/plugin.json` at the workspace
root (via `build.rs`). This is the plugin marketplace distribution path, but the
marketplace workflow is not yet established. The `.claude-plugin/` directory is
gitignored and won't ship via git clone.

For personal use, `acd install` is the recommended path.

## How It Works

### Plugin Manifest

The `build.rs` script generates `.claude-plugin/plugin.json` declaring all 6
hooks:

```json
{
  "name": "agent-console-dashboard",
  "version": "0.1.0",
  "hooks": {
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "acd claude-hook attention",
            "timeout": 10
          }
        ]
      }
    ],
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "acd claude-hook attention",
            "timeout": 10
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "acd claude-hook working",
            "timeout": 10
          }
        ]
      }
    ],
    "SessionEnd": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "acd claude-hook closed",
            "timeout": 10
          }
        ]
      }
    ],
    "Notification": [
      {
        "matcher": "elicitation_dialog",
        "hooks": [
          {
            "type": "command",
            "command": "acd claude-hook question",
            "timeout": 10
          }
        ]
      },
      {
        "matcher": "permission_prompt",
        "hooks": [
          {
            "type": "command",
            "command": "acd claude-hook attention",
            "timeout": 10
          }
        ]
      }
    ]
  }
}
```

### `acd claude-hook` Subcommand

When Claude Code fires a hook, it invokes `acd claude-hook <status>`. The
subcommand:

1. Reads JSON from stdin (Claude Code hook payload)
2. Extracts `session_id` from the JSON
3. Sends a SET command to the daemon via Unix socket
4. Outputs `{"continue": true}` on stdout (Claude Code protocol)

Valid status values: `attention`, `working`, `closed`, `question`.

Exit codes per the Claude Code hook spec:

| Code | Meaning        | When                                    |
| ---- | -------------- | --------------------------------------- |
| 0    | Success        | Status forwarded, or daemon not running |
| 2    | Blocking error | Malformed JSON on stdin                 |

When the daemon is not running, the subcommand exits 0 with a `systemMessage`
field so Claude Code isn't blocked but the condition is visible.

### Hook JSON Stdin Format

Claude Code passes JSON via stdin to hook commands. All hooks receive these
common fields (per the [hooks reference]):

| Field             | Type   | Required | Description                            |
| ----------------- | ------ | -------- | -------------------------------------- |
| `session_id`      | string | yes      | Unique session identifier              |
| `cwd`             | string | yes      | Working directory when hook is invoked |
| `transcript_path` | string | yes      | Path to conversation transcript        |
| `permission_mode` | string | yes      | Current permission mode                |
| `hook_event_name` | string | yes      | Which hook fired (e.g., `Stop`)        |

All five common fields are **always present** -- they are not optional. Some
hook events send additional fields (e.g., `tool_name` for `PreToolUse`), but ACD
does not use those.

ACD parses `session_id` and `cwd`. Unknown fields are silently ignored for
forward-compatibility with future Claude Code versions.

[hooks reference]: https://code.claude.com/docs/en/hooks

## Version Sync

The `build.rs` script generates `plugin.json` with the version from
`Cargo.toml`. If the versions diverge, the build fails. This ensures the plugin
version always matches the binary version.

## Troubleshooting

### Hooks Not Firing

- Verify hooks are installed: check `~/.claude/settings.json` for ACD entries
- Check `acd` is in PATH: `which acd`
- Verify Claude Code version supports hooks

### Dashboard Not Updating

- Confirm the daemon is running: `acd status`
- Test the hook subcommand manually:
  `echo '{"session_id":"test","cwd":"/tmp"}' | acd claude-hook attention`
- Check daemon logs for errors

### Manual Testing

```sh
# Start daemon
acd daemon &

# Simulate SessionStart
echo '{"session_id":"test-session","cwd":"/tmp"}' | acd claude-hook attention

# Simulate UserPromptSubmit
echo '{"session_id":"test-session","cwd":"/tmp"}' | acd claude-hook working

# Simulate Notification (elicitation_dialog)
echo '{"session_id":"test-session","cwd":"/tmp"}' | acd claude-hook question

# Simulate SessionEnd
echo '{"session_id":"test-session","cwd":"/tmp"}' | acd claude-hook closed

# Verify session appeared and status changes
acd status

# Clean up
acd set test-session closed
```
