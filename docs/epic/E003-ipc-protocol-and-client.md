# Epic: IPC Protocol & Client

**Epic ID:** E003
**Status:** Draft
**Priority:** High
**Estimated Effort:** M

## Summary

Define and implement the Inter-Process Communication (IPC) protocol for communication between hooks, CLI clients, and the daemon over Unix sockets. This epic establishes the text-based message protocol and provides CLI commands for interacting with the daemon, enabling both session status updates from hooks and queries from users.

## Goals

- Define a simple, text-based IPC message protocol for daemon communication
- Implement core commands (SET, LIST, SUBSCRIBE) for session management
- Create user-friendly CLI client commands for interacting with the daemon
- Enable real-time subscription to session updates for dashboards

## User Value

Users can interact with the Agent Console system through intuitive CLI commands. Hooks can reliably report session status changes using simple commands. Dashboards receive real-time updates via subscription, ensuring instant visibility into session state changes. The text-based protocol makes debugging and manual testing straightforward.

## Stories

| Story ID | Title | Priority | Status |
|----------|-------|----------|--------|
| [S009](../stories/S009-define-ipc-message-protocol.md) | Define IPC message protocol | P1 | Draft |
| [S010](../stories/S010-implement-set-command.md) | Implement SET command | P1 | Draft |
| [S011](../stories/S011-implement-list-command.md) | Implement LIST command | P1 | Draft |
| [S012](../stories/S012-implement-subscribe-command.md) | Implement SUBSCRIBE command | P1 | Draft |
| [S013](../stories/S013-create-cli-client-commands.md) | Create CLI client commands | P1 | Draft |

## Dependencies

- [E001 - Daemon Core Infrastructure](./E001-daemon-core-infrastructure.md) - Requires Unix socket server
- [E002 - Session Management](./E002-session-management.md) - Requires session data model and store

## Acceptance Criteria

- [ ] IPC protocol specification is documented and implemented
- [ ] SET command correctly updates session status in the daemon
- [ ] LIST command returns all current session states as JSON
- [ ] SUBSCRIBE command streams real-time session updates to clients
- [ ] CLI commands provide intuitive interface for all daemon operations
- [ ] Update latency is under 1ms target

## Technical Notes

### IPC Protocol

Text-based protocol over Unix socket (`/tmp/agent-console.sock`):

```text
# Commands (client → daemon)
SET <session> <status> [metadata_json]
RM <session>
LIST
SUBSCRIBE
RESURRECT <session>
API_USAGE <session> <tokens_json>

# Responses (daemon → client)
OK
OK <data_json>
ERR <message>
STATE <json>
UPDATE <session> <status> <elapsed_seconds>
```

### CLI Client Commands

```bash
# Update session status (called by hooks)
agent-console set <session> working
agent-console set <session> attention
agent-console set <session> question

# Remove session
agent-console rm <session>

# Query all sessions (one-shot)
agent-console list

# Subscribe to updates (streaming)
agent-console watch

# Resurrect closed session
agent-console resurrect <session>

# Report API usage
agent-console api-usage <session> --input 1000 --output 500
```

### Project Structure

```text
src/
├── daemon/
│   └── protocol.rs   # IPC message parsing
├── client/
│   ├── mod.rs
│   └── commands.rs   # CLI client commands
```

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| clap | CLI argument parsing |
| serde_json | JSON serialization for messages |
| tokio | Async socket communication |

### Design Decisions

- **Text-based protocol** - Human-readable for easy debugging
- **JSON for complex data** - Status updates use simple text, complex data uses JSON
- **Push model for subscriptions** - Server pushes updates, clients don't poll
- **Newline-delimited messages** - Simple framing for streaming
