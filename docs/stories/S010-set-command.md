# Story: Implement SET Command

**Story ID:** S010
**Epic:** [E003 - IPC Protocol & Client](../epic/E003-ipc-protocol-and-client.md)
**Status:** Draft
**Priority:** P1
**Estimated Points:** 3

## Description

As a hook script,
I want to send SET commands to update session status,
So that the daemon knows the current state of each agent session.

## Context

The SET command is the primary mechanism for hooks to report session status changes to the daemon. When Claude Code enters a new state (working, waiting for attention, asking a question), hooks call `agent-console set <session> <status>` to update the daemon. This is the write path for the entire system - all session state originates from SET commands.

The SET command must be fast (< 1ms) since it's called from hooks that execute during Claude Code's normal operation. It should not block or delay the agent's work.

## Implementation Details

### Technical Approach

1. Add SET command handler in daemon server
2. Parse session ID, status, and optional metadata from command
3. Update or create session in the store
4. Track state transition with timestamp
5. Broadcast update to all subscribed clients
6. Return OK response to sender

### Protocol Format

```text
# Request
SET <session> <status> [metadata_json]

# Examples
SET session-abc123 working
SET session-abc123 attention
SET session-abc123 question {"prompt": "Continue?"}

# Response
OK
ERR <message>
```

### Files to Modify

- `src/daemon/server.rs` - Add SET command handler
- `src/daemon/store.rs` - Implement session upsert logic
- `src/daemon/protocol.rs` - Ensure SET parsing is complete

### Dependencies

- [S009 - IPC Message Protocol](./S009-ipc-message-protocol.md) - Protocol definition
- [S003 - In-Memory Session Store](./S003-in-memory-session-store.md) - Store to update
- [S006 - Session Status Transitions](./S006-session-status-transitions.md) - Transition tracking

## Acceptance Criteria

- [ ] Given a new session ID, when SET is called, then a new session is created in the store
- [ ] Given an existing session ID, when SET is called, then the session status is updated
- [ ] Given a status change, when SET is called, then the previous status and duration are recorded in history
- [ ] Given a SET command with metadata, when processed, then metadata is stored with the session
- [ ] Given a successful SET, when complete, then "OK" is returned to the client
- [ ] Given an invalid status value, when SET is called, then "ERR invalid status" is returned
- [ ] Given any SET command, when processed, then all subscribed clients receive an UPDATE message
- [ ] Given typical usage, then SET command latency is under 1ms

## Testing Requirements

- [ ] Unit test: SET creates new session if not exists
- [ ] Unit test: SET updates existing session status
- [ ] Unit test: SET records state transition with timestamp
- [ ] Unit test: SET with invalid status returns error
- [ ] Integration test: SET broadcasts UPDATE to subscribers
- [ ] Integration test: SET command round-trip under 1ms
- [ ] Integration test: Concurrent SET commands are serialized correctly

## Out of Scope

- Session removal (RM command, separate story)
- API usage tracking (API_USAGE command)
- Session resurrection (RESURRECT command)
- Working directory detection (part of hook implementation)

## Notes

### Status Values

```rust
enum Status {
    Working,   // Agent is actively processing
    Attention, // Agent finished, needs user attention
    Question,  // Agent is asking a question
    Closed,    // Session has been closed (for resurrection)
}
```

### Broadcast to Subscribers

When a SET command is processed, the daemon must notify all subscribed clients:

```text
UPDATE session-abc123 attention 45
```

Where `45` is the number of seconds spent in the previous status.

### Metadata Use Cases

Optional metadata can include:
- `working_dir` - Current working directory of the session
- `prompt` - Question being asked (for question status)
- `session_id` - Claude Code session ID for resume capability

### Performance Considerations

- Use write lock only during store update, release before broadcast
- Broadcast asynchronously to not block SET response
- Consider batching broadcasts if many SETs arrive simultaneously
