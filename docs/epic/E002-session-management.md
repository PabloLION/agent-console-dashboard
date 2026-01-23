# Epic: Session Management

**Epic ID:** E002
**Status:** Draft
**Priority:** High
**Estimated Effort:** M

## Summary

Implement comprehensive session management for tracking multiple Claude Code agent sessions with real-time status updates, state history tracking, and lifecycle event handling. This epic provides the core domain model and business logic for managing session state within the daemon.

## Goals

- Define a robust session data model that captures all necessary session information
- Implement clear status transitions between Working, Attention, and Question states
- Track session state history to show transition timeline per session
- Handle session lifecycle events from creation through closure

## User Value

Users can monitor all their active Claude Code sessions in one place with real-time status visibility. The clear status indicators (Working, Attention, Question) immediately communicate which sessions need attention. State history allows users to understand session activity patterns and identify sessions that have been waiting too long.

## Stories

| Story ID | Title | Priority | Status |
|----------|-------|----------|--------|
| [S005](../stories/S005-session-data-model.md) | Define session data model | P1 | Draft |
| [S006](../stories/S006-session-status-transitions.md) | Implement session status transitions | P1 | Draft |
| [S007](../stories/S007-session-state-history.md) | Track session state history | P2 | Draft |
| [S008](../stories/S008-session-lifecycle-events.md) | Handle session lifecycle events | P1 | Draft |

## Dependencies

- [E001 - Daemon Core Infrastructure](./E001-daemon-core-infrastructure.md) - Requires the in-memory session store and daemon process

## Acceptance Criteria

- [ ] Session data model correctly represents all session properties (ID, status, working directory, agent type, timestamps)
- [ ] Status transitions are validated (only valid transitions allowed)
- [ ] State history records all transitions with timestamps and durations
- [ ] Session lifecycle events (create, update, close) are handled correctly
- [ ] Closed sessions retain metadata for potential resurrection

## Technical Notes

### Session Status States

| Status | Meaning | Triggered By |
|--------|---------|--------------|
| Working | Agent is processing | UserPromptSubmit hook |
| Attention | Agent stopped, waiting for user | Stop hook, Notification hook |
| Question | Agent asked a question | AskQuestion hook |
| Closed | Session ended | Session termination |

### Data Model

```rust
struct Session {
    id: String,
    agent_type: AgentType,       // ClaudeCode, Future agents
    status: Status,
    working_dir: PathBuf,
    since: Instant,              // When status last changed
    history: Vec<StateTransition>,
    api_usage: Option<ApiUsage>,
    closed: bool,                // For resurrection feature
    session_id: Option<String>,  // Claude Code session ID for resume
}

struct StateTransition {
    timestamp: Instant,
    from: Status,
    to: Status,
    duration: Duration,
}
```

### State History Display

- Show last N state transitions per session
- Expandable to see full history
- Each transition shows: timestamp, from/to states, duration in previous state

### Key Implementation Notes

- State is ephemeral (not persisted across reboots)
- History depth configurable via configuration
- Durations calculated automatically on state change
- Hook-based approach for receiving status updates is acceptable and practical
