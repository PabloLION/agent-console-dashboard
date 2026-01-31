# Epic: Claude Code Integration

**Epic ID:** E006 **Status:** Draft **Priority:** High **Estimated Effort:** M

## Summary

Implement the hook-based integration with Claude Code that allows the Agent
Console Dashboard to receive real-time status updates from active coding
sessions. This epic creates the shell scripts that Claude Code invokes at key
lifecycle events, enabling automatic session status updates without manual
intervention.

## Goals

- Create hook scripts that Claude Code invokes during session lifecycle events
- Enable automatic status updates when sessions start, stop, or require
  attention
- Provide seamless integration requiring minimal user configuration
- Document the hook registration process for easy setup

## User Value

Users get automatic, real-time dashboard updates without manually tracking
session states. When Claude Code stops working or sends a notification, the
dashboard immediately reflects this with an "Attention" status. When users send
new prompts, the session shows as "Working". This eliminates the cognitive
overhead of remembering which sessions need attention and enables efficient
multi-agent workflows.

## Stories

| Story ID                                                  | Title                                       | Priority | Status |
| --------------------------------------------------------- | ------------------------------------------- | -------- | ------ |
| [S006.01](../stories/S006.01-stop-hook-script.md)         | Create stop hook script                     | P1       | Draft  |
| [S006.02](../stories/S006.02-user-prompt-submit-hook.md)  | Create user-prompt-submit hook script       | P1       | Draft  |
| [S006.03](../stories/S006.03-notification-hook-script.md) | Create notification hook script             | P1       | Draft  |
| [S006.04](../stories/S006.04-hook-registration-docs.md)   | Document hook registration in settings.json | P2       | Draft  |

## Dependencies

- [E001 - Daemon Core Infrastructure](./E001-daemon-core-infrastructure.md) -
  Daemon must be running to receive hook commands
- [E003 - IPC Protocol & Client](./E003-ipc-protocol-and-client.md) - CLI client
  needed for hook scripts to communicate with daemon

## Acceptance Criteria

- [ ] Stop hook script sets session status to "Attention" when Claude Code stops
- [ ] User-prompt-submit hook script sets session status to "Working" when user
      sends a message
- [ ] Notification hook script sets session status to "Attention" when Claude
      sends a notification
- [ ] Hook scripts correctly extract session_id from JSON stdin
- [ ] Documentation covers complete hook registration in Claude Code settings
- [ ] All hooks are portable bash scripts with minimal dependencies
- [ ] Manual test plan for each hook type documented per
      [testing strategy](../decisions/testing-strategy.md)

## Technical Notes

### Hook Architecture

Claude Code hooks are shell scripts invoked at specific lifecycle events:

| Hook               | When Fired                   | Dashboard Action      |
| ------------------ | ---------------------------- | --------------------- |
| `Stop`             | Session stops/completes      | Set status: Attention |
| `Notification`     | Claude sends notification    | Set status: Attention |
| `UserPromptSubmit` | User sends message           | Set status: Working   |
| `PreToolUse`       | AskUserQuestion tool invoked | Set status: Question  |

### Hook Script Location

Scripts are placed in `~/.claude/hooks/` (or a user-configured path):

```text
~/.claude/hooks/
├── stop.sh
├── user-prompt-submit.sh
└── notification.sh
```

### Hook Registration

Hooks are registered in `~/.claude/settings.json`:

```json
{
  "hooks": {
    "Stop": ["~/.claude/hooks/stop.sh"],
    "UserPromptSubmit": ["~/.claude/hooks/user-prompt-submit.sh"],
    "Notification": ["~/.claude/hooks/notification.sh"]
  }
}
```

### Hook JSON Stdin

Claude Code passes JSON data via stdin to ALL hook types. Every hook receives:

| Field             | Type   | Description                     |
| ----------------- | ------ | ------------------------------- |
| `session_id`      | string | Unique session identifier       |
| `cwd`             | string | Current working directory       |
| `transcript_path` | string | Path to conversation transcript |
| `permission_mode` | string | Current permission mode         |
| `hook_event_name` | string | Which hook fired (Stop, etc.)   |

Plus event-specific fields (e.g., `prompt` for UserPromptSubmit, `message` for
Notification).

See [D8](../architecture/2026-01-31-discussion-decisions.md) for full details.

### Hook Script Pattern

Each hook script parses JSON stdin to extract `session_id`:

```bash
#!/bin/bash
INPUT=$(cat)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id')
agent-console set "$SESSION_ID" <status>
```

### AskUserQuestion Hook (Resolved)

**AskUserQuestion IS detectable** via `PreToolUse` hook with `AskUserQuestion`
matcher (since Claude Code v2.0.76, January 4, 2026). See
[Q7 decision](../plans/7-decisions.md#q7-askuserquestion-hook-detection) for
full details.

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "AskUserQuestion",
        "hooks": [
          {
            "type": "command",
            "command": "agent-console set $CC_SESSION_ID question"
          }
        ]
      }
    ]
  }
}
```

**Minimum version:** Claude Code v2.0.76+

**Known bug:** [#15400](https://github.com/anthropics/claude-code/issues/15400)
— PermissionRequest hook incorrectly interferes with AskUserQuestion.

### Configuration Integration

Hooks can be configured via the main config file:

```toml
[integrations.claude-code]
enabled = true
hooks_path = "~/.claude/hooks"
```

### Testing Strategy

Manual testing workflow:

1. Start daemon: `agent-console daemon`
2. Start dashboard: `agent-console tui`
3. Configure hooks in Claude Code settings
4. Start Claude Code session and interact
5. Verify dashboard updates on each hook event
