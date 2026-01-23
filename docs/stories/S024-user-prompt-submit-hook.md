# Story: Create User Prompt Submit Hook Script

**Story ID:** S024
**Epic:** [E006 - Claude Code Integration](../epic/E006-claude-code-integration.md)
**Status:** Draft
**Priority:** P1
**Estimated Points:** 2

## Description

As a user,
I want a hook script that runs when I submit a prompt to Claude Code,
So that my dashboard shows which sessions are actively working.

## Context

The UserPromptSubmit hook is fired by Claude Code whenever the user sends a new message or prompt. This signals that Claude is now processing the request and actively working. The dashboard should immediately reflect this state change by setting the session status to "Working".

This hook is essential for the core workflow: when managing multiple sessions, users need to know which ones are actively processing versus waiting for input. The "Working" status (displayed as `-` in green) indicates the session is busy and doesn't need attention.

## Implementation Details

### Technical Approach

1. Create `scripts/hooks/user-prompt-submit.sh` (template for users to copy)
2. Script extracts project name from `$PWD` using `basename`
3. Script calls `agent-console set <project> working` to update status
4. Script must be executable and use `/bin/bash` shebang
5. Include comments explaining usage and customization options

### Files to Modify

- `scripts/hooks/user-prompt-submit.sh` - Create the user-prompt-submit hook script template
- `scripts/hooks/README.md` - Update hook documentation (if exists)

### Dependencies

- [E001 - Daemon Core Infrastructure](../epic/E001-daemon-core-infrastructure.md) - Daemon must be running
- [S010 - SET Command](./S010-set-command.md) - CLI SET command must work
- [S013 - CLI Client Commands](./S013-cli-client-commands.md) - agent-console CLI must be installed
- [S023 - Stop Hook Script](./S023-stop-hook-script.md) - Follows same pattern

## Acceptance Criteria

- [ ] Given the user-prompt-submit.sh script exists, when copied to `~/.claude/hooks/`, then it is ready to use
- [ ] Given Claude Code invokes the hook, when the script runs, then `agent-console set <project> working` is called
- [ ] Given Claude Code is running in `/home/user/projects/api-server`, when user submits prompt, then project name is `api-server`
- [ ] Given the agent-console daemon is not running, when hook is invoked, then script fails gracefully (no crash)
- [ ] Given a session was in "Attention" status, when user submits a prompt, then status changes to "Working"
- [ ] Given the script runs, when completed, then it exits with status 0 on success

## Testing Requirements

- [ ] Manual test: Copy script to `~/.claude/hooks/user-prompt-submit.sh` and verify it executes
- [ ] Manual test: Run script directly from a project directory and verify dashboard shows "Working"
- [ ] Manual test: Verify status transition from Attention to Working in dashboard
- [ ] Manual test: Run script when daemon is not running to verify graceful failure

## Out of Scope

- Tracking the content of the user's prompt
- Rate limiting multiple rapid submissions
- Custom status messages beyond "working"
- Windows support (bash scripts only)

## Notes

### Script Template

```bash
#!/bin/bash
# Agent Console Dashboard - UserPromptSubmit Hook for Claude Code
#
# This script is invoked when a user submits a prompt to Claude Code.
# It sets the session status to "Working" in the dashboard.
#
# Installation:
#   1. Copy this file to ~/.claude/hooks/user-prompt-submit.sh
#   2. Make it executable: chmod +x ~/.claude/hooks/user-prompt-submit.sh
#   3. Register in ~/.claude/settings.json (see S026)
#
# Customization:
#   - Modify PROJECT derivation for custom naming
#   - Add timestamp logging for session activity tracking

PROJECT=$(basename "$PWD")
agent-console set "$PROJECT" working
```

### Status Transition Flow

```
[Any Status] --user submits prompt--> [Working]
[Working] --claude stops/completes--> [Attention]
```

This hook creates the first half of the automatic status cycling. Combined with the Stop hook (S023), it provides a complete picture of session activity:

1. User sends prompt -> Status: Working (green `-`)
2. Claude processes and responds
3. Claude stops/completes -> Status: Attention (yellow time elapsed)
4. User sends next prompt -> Back to Working

### Performance Considerations

This hook may be called frequently during active sessions. The script should:

- Execute quickly (avoid unnecessary operations)
- Not block Claude Code's operation
- Handle rapid successive calls gracefully

### Hook Event Data

Claude Code may pass additional data to hooks via environment variables or stdin. Future versions could capture:

- Message length
- Timestamp
- Session ID

For now, the script ignores any additional data and just updates the status.
