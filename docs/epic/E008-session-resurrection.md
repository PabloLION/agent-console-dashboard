# Epic: Session Resurrection

**Epic ID:** E008 **Status:** Draft **Priority:** Medium **Estimated Effort:** S

## Summary

Define the IPC protocol and metadata storage for session resurrection. This epic
handles the daemon-side interface (commands, metadata retention) so that
frontends (TUI, Zellij via E010) can implement the actual terminal creation.

## Goals

- Retain session metadata when Claude Code sessions close (working directory,
  session ID)
- Define RESURRECT and LIST_CLOSED IPC commands
- Provide metadata storage for closed sessions
- Leave terminal/pane creation to E010 (Zellij) or TUI

## User Value

Users frequently close terminal sessions or panes only to realize they need to
continue a previous Claude Code conversation. Instead of losing that context,
users can see their recently closed sessions and resurrect them with a single
command. This preserves valuable conversation history and context, reducing
friction when switching between tasks or recovering from accidental closures.

## Stories

| Story ID                                                   | Title                                           | Priority | Status |
| ---------------------------------------------------------- | ----------------------------------------------- | -------- | ------ |
| [S008.01](../stories/S008.01-closed-session-metadata.md)   | Store session metadata for closed sessions      | P1       | Draft  |
| [S008.02](../stories/S008.02-resurrect-command.md)         | Implement RESURRECT command                     | P1       | Draft  |
| [S008.03](../stories/S008.03-claude-resume-integration.md) | ~~claude --resume integration~~ (moved to E010) | —        | Moved  |

## Dependencies

- [E001 - Daemon Core Infrastructure](./E001-daemon-core-infrastructure.md) -
  Daemon must be running to store and retrieve session metadata
- [E002 - Session Management](./E002-session-management.md) - Session lifecycle
  events must be tracked to know when sessions close
- [E003 - IPC Protocol & Client](./E003-ipc-protocol-and-client.md) - RESURRECT
  command requires IPC protocol support
- [E006 - Claude Code Integration](./E006-claude-code-integration.md) - Hook
  scripts detect session start/stop events

## Acceptance Criteria

- [ ] Session metadata (working directory, session ID, timestamp) is retained
      after session closes
- [ ] Users can list previously closed sessions that are eligible for
      resurrection
- [ ] RESURRECT command returns session metadata for frontends to act on
- [ ] Session metadata is in-memory only, lost on daemon restart
- [ ] Resumability is a user-settable flag (auto-detection not available)
- [ ] Multiple closed sessions per directory are listed individually
- [ ] Resurrection validates working directory still exists before proceeding
- [ ] Unit tests for metadata storage; integration tests for RESURRECT command
      per [testing strategy](../decisions/testing-strategy.md)

## Technical Notes

### Session Metadata Storage

When a session closes, retain:

| Field       | Description                                                 |
| ----------- | ----------------------------------------------------------- |
| session_id  | Claude Code session identifier                              |
| working_dir | Directory where session was running                         |
| closed_at   | Timestamp when session closed                               |
| resumable   | Whether session can be resumed (context limit not exceeded) |

### Resurrection Mechanism

1. Track session metadata when session starts via hooks
2. When session closes (Stop hook), mark as "closed" but retain metadata
3. User selects closed session from dashboard or CLI
4. Dashboard/CLI invokes `claude --resume <session-id>` in appropriate context

### Challenges & Considerations

**Terminal Context:**

- Terminal/pane creation is out of scope for this epic (handled by E010)
- This epic provides the protocol; frontends decide how to open sessions

**Context Limits:**

- Claude Code sessions may exceed context limits and become non-resumable
- Should detect this condition and inform user
- Mark such sessions as "not resumable" in the listing

**Working Directory:**

- Resurrected session should open in the original working directory
- Validate directory still exists before resurrection

**Multiple Sessions Per Directory:**

- Multiple Claude Code sessions may share the same working directory
- Per [Q19 decision](../plans/7-decisions.md#q19-resurrection-mechanism), v0/v1
  relies on Claude Code's built-in session picker when multiple sessions exist
- Display session count per directory so users know disambiguation will be
  needed
- v2+ may use `claude --resume <session-id>` for precise resurrection

### IPC Commands

New commands to support:

```text
RESURRECT <session-id>
  Attempt to resume a closed session
  Returns: OK | ERROR <reason>

LIST_CLOSED
  List closed sessions eligible for resurrection
  Returns: JSON array of closed session metadata
```

### Scope Boundary with E010

This epic defines the **protocol and metadata**. E010 (Zellij Integration)
handles the **terminal/pane creation** — invoking `claude --resume <session-id>`
in the appropriate context.

### Testing Strategy

- Unit tests for session metadata storage and retrieval
- Unit tests for resurrection eligibility checks
- Integration tests for RESURRECT command
- Integration tests with `claude --resume` invocation
- Manual testing for terminal pane creation scenarios
