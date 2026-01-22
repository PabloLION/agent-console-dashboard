# Features

Complete feature list organized by priority.

---

## Core Features (Now)

### Session Management

Track multiple Claude Code sessions with real-time status updates.

| Status    | Meaning                         | Triggered By                 |
| --------- | ------------------------------- | ---------------------------- |
| Working   | Agent is processing             | UserPromptSubmit hook        |
| Attention | Agent stopped, waiting for user | Stop hook, Notification hook |
| Question  | Agent asked a question          | AskQuestion hook             |

**Implementation:** Uses Claude Code hooks to receive status changes. Hook-based approach is acceptable and practical.

### API Usage Display

Show API consumption metrics for visibility into costs and limits.

- Current session token usage
- Cumulative usage across sessions
- Rate limit status (if available)

### Session State History

Track state transitions over time (NOT chat history).

| Field     | Description                  |
| --------- | ---------------------------- |
| Timestamp | When state changed           |
| From      | Previous status              |
| To        | New status                   |
| Duration  | Time spent in previous state |

Display: Show last N state transitions per session. Expandable to see full history.

### Centralized Configuration

Single config file for all settings.

Location: `~/.config/agent-console/config.toml` (or similar)

```toml
[ui]
layout = "two-line"  # or "one-line", "custom"
widgets = ["working-dir", "status", "api-usage"]

[agents.claude-code]
enabled = true
hooks_path = "~/.claude/hooks"

[integrations.zellij]
enabled = true
```

### Session Resurrection

Reopen/invoke closed Claude Code sessions.

**Mechanism:**

1. Track session metadata when session starts (working directory, session ID)
2. When session closes, mark as "closed" but retain metadata
3. User can select closed session and "resurrect" it
4. Dashboard invokes `claude --resume <session-id>` in appropriate context

**Challenges:**

- Need to know where to open the terminal (which pane/tab)
- Session may have exceeded context limit (not resumable)
- May need Zellij integration to create new pane

### AskQuestion Hook Support

Properly handle Claude Code's AskUserQuestion tool.

**Current Problem:** No hook fires when Claude asks a question.

**Solution:** Investigate Claude Code hooks, may need to use a different hook or request new hook type.

---

## Later Features

### Multi-Agent Support

Support agents beyond Claude Code that implement a hook interface.

**Requirements for agent support:**

- Agent must have hooks or similar callback mechanism
- Agent must expose session state (working/attention/question)
- Agent must provide session identifier

**Potential agents:**

- Other Claude-based tools
- Custom AI agents with hook support
- Any tool that can call shell scripts on state change

---

## Non-Goals

These are explicitly out of scope:

| Non-Goal                  | Reason                                      |
| ------------------------- | ------------------------------------------- |
| Chat history display      | Too complex, privacy concerns, out of scope |
| Remote/network access     | Local IPC only, security                    |
| Persistence across reboot | State is ephemeral, intentional             |
| Complex data structures   | Keep it simple, key-value is sufficient     |
