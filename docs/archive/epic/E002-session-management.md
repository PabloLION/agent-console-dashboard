# Epic: Session Management

**Epic ID:** E002 **Status:** Draft **Priority:** High **Estimated Effort:** M

## Summary

Implement comprehensive session management for tracking multiple Claude Code
agent sessions with real-time status updates, state history tracking, and
lifecycle event handling. This epic provides the core domain model and business
logic for managing session state within the daemon.

## Goals

- Define a robust session data model that captures all necessary session
  information
- Implement clear status transitions between Working, Attention, and Closed
  states
- Track session state history to show transition timeline per session
- Handle session lifecycle events from creation through closure

## User Value

Users can monitor all their active Claude Code sessions in one place with
real-time status visibility. The clear status indicators (Working, Attention,
Attention) immediately communicate which sessions need attention. State history
allows users to understand session activity patterns and identify sessions that
have been waiting too long.

## Stories

| Story ID                                                    | Title                                | Priority | Status |
| ----------------------------------------------------------- | ------------------------------------ | -------- | ------ |
| [S002.01](../stories/S002.01-session-data-model.md)         | Define session data model            | P1       | Draft  |
| [S002.02](../stories/S002.02-session-status-transitions.md) | Implement session status transitions | P1       | Draft  |
| [S002.03](../stories/S002.03-session-state-history.md)      | Track session state history          | P2       | Draft  |
| [S002.04](../stories/S002.04-session-lifecycle-events.md)   | Handle session lifecycle events      | P1       | Draft  |

## Dependencies

- [E001 - Daemon Core Infrastructure](./E001-daemon-core-infrastructure.md) -
  Requires the in-memory session store and daemon process

## Acceptance Criteria

- [ ] Session data model correctly represents all session properties (ID,
      status, working directory, agent type, timestamps)
- [ ] Status transitions are validated (only valid transitions allowed)
- [ ] State history records all transitions with timestamps and durations
- [ ] Session lifecycle events (create, update, close) are handled correctly
- [ ] Closed sessions retain metadata for potential resurrection
- [ ] Unit tests for status transitions and state history per
      [testing strategy](../decisions/testing-strategy.md)

## Technical Notes

### Session Status States

| Status    | Meaning                         | Triggered By                                       |
| --------- | ------------------------------- | -------------------------------------------------- |
| Working   | Agent is processing             | UserPromptSubmit hook                              |
| Attention | Agent stopped, waiting for user | Stop hook, Notification hook, AskUserQuestion hook |
| Closed    | Session ended                   | Session termination                                |

### Session Identification

Sessions are identified by `session_id` from Claude Code's JSON stdin, available
in ALL hook types (Stop, UserPromptSubmit, Notification, PreToolUse,
SessionStart, SessionEnd). See [Q16](../plans/7-decisions.md) and
[D8](../architecture/2026-01-31-discussion-decisions.md).

Hook scripts extract session_id from stdin:

```bash
INPUT=$(cat)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id')
agent-console set "$SESSION_ID" working
```

Available fields in hook JSON stdin: `session_id`, `cwd`, `transcript_path`,
`permission_mode`, `hook_event_name` (plus event-specific fields).

**Session auto-creation:** First SET for an unknown session_id creates the
session automatically (Q17). No explicit registration step needed.

### State Transition Rules

| From      | To        | Valid? | Trigger                                            |
| --------- | --------- | ------ | -------------------------------------------------- |
| Working   | Attention | Yes    | Stop hook, Notification hook, AskUserQuestion hook |
| Working   | Closed    | Yes    | SessionEnd hook                                    |
| Attention | Working   | Yes    | UserPromptSubmit hook                              |
| Attention | Closed    | Yes    | SessionEnd hook                                    |
| Closed    | Working   | Yes    | Resurrection only (via RESURRECT command)          |
| Closed    | \*        | No     | Cannot transition except via resurrection          |

Same-status transitions (e.g., Working → Working) update `since` timestamp but
do not record a new history entry.

### Data Model

```rust
struct Session {
    id: String,              // From session_id in JSON stdin
    agent_type: AgentType,   // ClaudeCode, Future agents
    status: Status,
    working_dir: PathBuf,    // From cwd in JSON stdin
    display_name: String,    // Derived from basename of cwd
    since: Instant,          // When status last changed
    history: Vec<StateTransition>,
    closed: bool,            // For resurrection feature
}

struct StateTransition {
    timestamp: Instant,
    from: Status,
    to: Status,
    duration: Duration,
}
```

**Metadata persistence:** Session state is in-memory only, lost on daemon
restart. This is intentional — sessions are volatile and re-register via hooks.
See [D1](../architecture/2026-01-31-discussion-decisions.md).

**Concurrency:** Single-threaded actor model processes one message at a time. No
race conditions on store access. See
[concurrency model](../architecture/concurrency.md).

### State History Display

- Show last N state transitions per session
- Expandable to see full history
- Each transition shows: timestamp, from/to states, duration in previous state

### Key Implementation Notes

- State is ephemeral (not persisted across reboots)
- History depth configurable via configuration
- Durations calculated automatically on state change
- Hook-based approach for receiving status updates is acceptable and practical

### Complexity Review Notes

The [complexity review](../decisions/complexity-review.md) identified types in
the current codebase that appear unused. These types ARE needed by this epic:

- `SessionMetadata` — used by E008 (session resurrection)
- Account-level API usage is handled separately by E011 (`claude-usage` crate),
  not stored per-session
- `StateTransition` — used by S002.03 (state history)
- `history_depth_limit` — used by S002.03 configuration

These complexity considerations are addressed during implementation of
individual stories. No separate tracking needed.
