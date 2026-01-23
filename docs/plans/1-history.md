# History: Evolution from CC-Hub

## Original Problem

Running multiple Claude Code sessions in Zellij requires visibility into which
sessions need user attention. Without a dashboard, users must manually check
each pane.

---

## CC-Hub v1: Bash/jq Implementation (Current)

### Components

| Script             | Location        | Purpose                             |
| ------------------ | --------------- | ----------------------------------- |
| `cc-hub-update`    | `~/.local/bin/` | Update session status               |
| `cc-hub-dashboard` | `~/.local/bin/` | Display dashboard (one-line format) |
| `cc-hub-remove`    | `~/.local/bin/` | Remove session from state           |

### State File

Location: `/tmp/cc-hub-state.json`

```json
{
  "project-name": {
    "status": "working|attention",
    "since": 1736870400
  }
}
```

### Hooks Integration

| Hook             | File                                          | Action                     |
| ---------------- | --------------------------------------------- | -------------------------- |
| Stop             | `~/.claude/pablo/hooks/stop.sh`               | Sets status to `attention` |
| Notification     | `~/.claude/pablo/hooks/notification.sh`       | Sets status to `attention` |
| UserPromptSubmit | `~/.claude/pablo/hooks/user-prompt-submit.sh` | Sets status to `working`   |

Hooks registered in `~/.claude/settings.json`.

### Dashboard Output Format

One-line format:

```text
proj-a: - | proj-b: 2m34s | proj-c: -
```

- `-` = working (no attention needed)
- `2m34s` = needs attention, showing elapsed time

### Zellij Integration

The `zellij-claude-layout` script starts `cc-hub-dashboard --watch` in the
dashboard pane.

---

## Known Issues (v1)

### UserPromptSubmit hook not resetting status

**Status:** Open

**Symptom:** When user sends a message, dashboard continues showing elapsed time
instead of resetting to `-`.

**Expected:** Hook should call `cc-hub-update <project> working`, resetting
timer.

### AskQuestion hook not working

**Status:** Open

**Symptom:** When Claude Code asks a question (AskUserQuestion tool), no hook
fires.

**Impact:** Dashboard doesn't show attention status for questions.

---

## Limitations (v1)

| Limitation         | Description                                          |
| ------------------ | ---------------------------------------------------- |
| File-based         | State persists to disk, not ideal for high-frequency |
| No daemon          | Each dashboard instance reads file independently     |
| Lost on reboot     | `/tmp` is cleared on system restart                  |
| No synchronization | Multiple dashboard instances are independent         |
| No session history | Only current status, no state transitions            |
| No resurrection    | Cannot reopen closed sessions                        |
| Claude Code only   | No support for other agents                          |

---

## Why Rewrite?

The bash implementation proved the concept but has fundamental limitations:

1. **Performance** - File I/O for every status check
2. **Architecture** - No shared state, no real-time updates
3. **Features** - Cannot add API monitoring, session history, resurrection
4. **Extensibility** - Hard to add new agents or UI features

The Rust rewrite addresses all of these while adding new capabilities.
