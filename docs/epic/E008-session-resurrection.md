# Epic: Session Resurrection

**Epic ID:** E008 **Status:** Draft **Priority:** Medium **Estimated Effort:** M

## Summary

Implement the ability to reopen and resume closed Claude Code sessions from the
dashboard. This feature tracks session metadata when sessions close, retains it
for later resurrection, and provides a command to invoke
`claude --resume <session-id>` in the appropriate context.

## Goals

- Retain session metadata when Claude Code sessions close (working directory,
  session ID)
- Enable users to view and select from previously closed sessions
- Provide seamless session resurrection through the dashboard interface
- Integrate with Claude Code's `--resume` flag for session continuation

## User Value

Users frequently close terminal sessions or panes only to realize they need to
continue a previous Claude Code conversation. Instead of losing that context,
users can see their recently closed sessions and resurrect them with a single
command. This preserves valuable conversation history and context, reducing
friction when switching between tasks or recovering from accidental closures.

## Stories

| Story ID                                             | Title                                      | Priority | Status |
| ---------------------------------------------------- | ------------------------------------------ | -------- | ------ |
| [S031](../stories/S031-closed-session-metadata.md)   | Store session metadata for closed sessions | P1       | Draft  |
| [S032](../stories/S032-resurrect-command.md)         | Implement RESURRECT command                | P1       | Draft  |
| [S033](../stories/S033-claude-resume-integration.md) | Integrate with claude --resume             | P2       | Draft  |

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
- [ ] RESURRECT command successfully invokes `claude --resume <session-id>`
- [ ] Sessions that have exceeded context limits are marked as not resumable
- [ ] Resurrection workflow handles terminal pane creation appropriately

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

- Need to determine where to open the resurrected session
- May require Zellij integration to create new pane
- Consider opening in current terminal if no multiplexer available

**Context Limits:**

- Claude Code sessions may exceed context limits and become non-resumable
- Should detect this condition and inform user
- Mark such sessions as "not resumable" in the listing

**Working Directory:**

- Resurrected session should open in the original working directory
- Validate directory still exists before resurrection

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

### Integration with Zellij

If Zellij integration (E010) is available:

- Optionally create a new pane for the resurrected session
- Position pane according to user preferences
- Fall back to current terminal if Zellij not available

### Testing Strategy

- Unit tests for session metadata storage and retrieval
- Unit tests for resurrection eligibility checks
- Integration tests for RESURRECT command
- Integration tests with `claude --resume` invocation
- Manual testing for terminal pane creation scenarios
