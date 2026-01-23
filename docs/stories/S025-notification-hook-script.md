# Story: Create Notification Hook Script

**Story ID:** S025
**Epic:** [E006 - Claude Code Integration](../epic/E006-claude-code-integration.md)
**Status:** Draft
**Priority:** P1
**Estimated Points:** 2

## Description

As a user,
I want a hook script that runs when Claude Code sends a notification,
So that my dashboard alerts me when Claude needs my attention mid-conversation.

## Context

The Notification hook is fired by Claude Code when it wants to notify the user about something important. This could be a question, a request for clarification, an error that needs attention, or a milestone completion. Unlike the Stop hook (which fires when processing ends), the Notification hook can fire mid-conversation.

The dashboard should set the session status to "Attention" when a notification occurs, signaling that the user should check on this session. This enables efficient multi-tasking across multiple Claude Code sessions.

## Implementation Details

### Technical Approach

1. Create `scripts/hooks/notification.sh` (template for users to copy)
2. Script extracts project name from `$PWD` using `basename`
3. Script calls `agent-console set <project> attention` to update status
4. Script must be executable and use `/bin/bash` shebang
5. Include comments explaining usage and customization options
6. Optionally trigger system notifications (commented out by default)

### Files to Modify

- `scripts/hooks/notification.sh` - Create the notification hook script template
- `scripts/hooks/README.md` - Update hook documentation (if exists)

### Dependencies

- [E001 - Daemon Core Infrastructure](../epic/E001-daemon-core-infrastructure.md) - Daemon must be running
- [S010 - SET Command](./S010-set-command.md) - CLI SET command must work
- [S013 - CLI Client Commands](./S013-cli-client-commands.md) - agent-console CLI must be installed
- [S023 - Stop Hook Script](./S023-stop-hook-script.md) - Follows same pattern

## Acceptance Criteria

- [ ] Given the notification.sh script exists, when copied to `~/.claude/hooks/`, then it is ready to use
- [ ] Given Claude Code sends a notification, when the hook runs, then `agent-console set <project> attention` is called
- [ ] Given Claude Code is running in `/home/user/code/frontend`, when notification fires, then project name is `frontend`
- [ ] Given the agent-console daemon is not running, when hook is invoked, then script fails gracefully (no crash)
- [ ] Given a session was in "Working" status, when notification fires, then status changes to "Attention"
- [ ] Given the script includes optional system notification code, when user enables it, then OS notifications appear

## Testing Requirements

- [ ] Manual test: Copy script to `~/.claude/hooks/notification.sh` and verify it executes
- [ ] Manual test: Trigger a notification in Claude Code and verify dashboard updates
- [ ] Manual test: Verify status transition from Working to Attention in dashboard
- [ ] Manual test: Run script when daemon is not running to verify graceful failure
- [ ] Manual test: Enable and test system notification integration (macOS/Linux)

## Out of Scope

- Capturing notification content or message
- Different status types based on notification type
- Notification history tracking
- Windows support (bash scripts only)

## Notes

### Script Template

```bash
#!/bin/bash
# Agent Console Dashboard - Notification Hook for Claude Code
#
# This script is invoked when Claude Code sends a notification.
# It sets the session status to "Attention" in the dashboard.
#
# Installation:
#   1. Copy this file to ~/.claude/hooks/notification.sh
#   2. Make it executable: chmod +x ~/.claude/hooks/notification.sh
#   3. Register in ~/.claude/settings.json (see S026)
#
# Customization:
#   - Uncomment system notification lines below for OS-level alerts
#   - Modify PROJECT derivation for custom naming

PROJECT=$(basename "$PWD")
agent-console set "$PROJECT" attention

# Optional: System notification (uncomment one based on your OS)
#
# macOS:
# osascript -e "display notification \"Claude needs attention\" with title \"$PROJECT\""
#
# Linux (requires notify-send):
# notify-send "$PROJECT" "Claude needs attention"
```

### Notification Types

Claude Code may send notifications for various reasons:

| Reason | Example |
|--------|---------|
| Question | "Should I proceed with the migration?" |
| Error | "Build failed with error X" |
| Permission | "Can I modify file Y?" |
| Completion | "Task completed successfully" |

Currently, all notification types result in the same "Attention" status. Future versions could differentiate:

- Question -> "Question" status (blue `?`)
- Error -> "Error" status (red `!`)
- Completion -> Keep "Attention" (yellow time)

### System Notification Integration

The script template includes commented-out examples for system notifications:

**macOS:**
```bash
osascript -e "display notification \"Claude needs attention\" with title \"$PROJECT\""
```

**Linux (GNOME/KDE):**
```bash
notify-send "$PROJECT" "Claude needs attention"
```

Users can enable these for additional visibility when working with multiple terminals or applications.

### Difference from Stop Hook

| Hook | When Fired | Typical Scenario |
|------|------------|------------------|
| Stop | Session ends | Claude finished all work |
| Notification | Mid-conversation | Claude has a question or alert |

Both hooks set status to "Attention", but they fire at different times. The Notification hook is more immediate - it signals that Claude is still in the conversation but needs user input.

### AskQuestion Hook Note

Claude Code may also have an AskQuestion hook that fires specifically when Claude uses the AskUserQuestion tool. This hook is currently not well-documented or may not be available. If it becomes available, a separate story should be created to handle it with a "Question" status.
