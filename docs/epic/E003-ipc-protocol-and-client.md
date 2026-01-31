# Epic: IPC Protocol & Client

**Epic ID:** E003 **Status:** Draft **Priority:** High **Estimated Effort:** M

## Summary

Define and implement the Inter-Process Communication (IPC) protocol for
communication between hooks, CLI clients, and the daemon over Unix sockets. This
epic establishes the text-based message protocol and provides CLI commands for
interacting with the daemon, enabling both session status updates from hooks and
queries from users.

## Goals

- Define a simple, text-based IPC message protocol for daemon communication
- Implement core commands (SET, LIST, SUBSCRIBE) for session management
- Create user-friendly CLI client commands for interacting with the daemon
- Enable real-time subscription to session updates for dashboards

## User Value

Users can interact with the Agent Console system through intuitive CLI commands.
Hooks can reliably report session status changes using simple commands.
Dashboards receive real-time updates via subscription, ensuring instant
visibility into session state changes. The text-based protocol makes debugging
and manual testing straightforward.

## Stories

| Story ID                                                     | Title                                 | Priority | Status |
| ------------------------------------------------------------ | ------------------------------------- | -------- | ------ |
| [S003.01](../stories/S003.01-ipc-message-protocol.md)        | Define IPC message protocol           | P1       | Draft  |
| [S003.02](../stories/S003.02-set-command.md)                 | Implement SET command                 | P1       | Draft  |
| [S003.03](../stories/S003.03-list-command.md)                | Implement LIST command                | P1       | Draft  |
| [S003.04](../stories/S003.04-subscribe-command.md)           | Implement SUBSCRIBE command           | P1       | Draft  |
| [S003.05](../stories/S003.05-cli-client-commands.md)         | Create CLI client commands            | P1       | Draft  |
| [S003.06](../stories/S003.06-client-module-internal-only.md) | Ensure client module remains internal | P3       | Draft  |

## Dependencies

- [E001 - Daemon Core Infrastructure](./E001-daemon-core-infrastructure.md) -
  Requires Unix socket server
- [E002 - Session Management](./E002-session-management.md) - Requires session
  data model and store

## Acceptance Criteria

- [ ] IPC protocol specification is documented and implemented
- [ ] SET command correctly updates session status in the daemon
- [ ] LIST command returns all current session states as JSON
- [ ] SUBSCRIBE command streams real-time session updates to clients
- [ ] CLI commands provide intuitive interface for all daemon operations
- [ ] Update latency is under 1ms target
- [ ] Unit tests for protocol parsing; integration tests for IPC commands per
      [testing strategy](../decisions/testing-strategy.md)

## Technical Notes

### IPC Protocol

JSON Lines protocol over Unix socket (`/tmp/agent-console.sock`), per
[D2 decision](../architecture/2026-01-31-discussion-decisions.md#d2-ipc-message-format---json-lines).
Each message is a single JSON object terminated by `\n`:

```text
# Commands (client → daemon) — one JSON object per line
{"type":"SET","session":"abc","status":"working","metadata":{"cwd":"/path"}}
{"type":"RM","session":"abc"}
{"type":"LIST"}
{"type":"SUBSCRIBE"}
{"type":"RESURRECT","session":"abc"}

# Responses (daemon → client) — one JSON object per line
{"type":"OK"}
{"type":"OK","data":[...]}
{"type":"ERR","message":"error description"}
{"type":"STATE","sessions":[...]}
{"type":"UPDATE","session":"abc","status":"working","elapsed":45}
```

### CLI Client Commands

```bash
# Update session status (called by hooks)
agent-console set <session> working
agent-console set <session> attention
agent-console set <session> attention

# Remove session
agent-console rm <session>

# Query all sessions (one-shot)
agent-console list

# Subscribe to updates (streaming)
agent-console watch

# Resurrect closed session
agent-console resurrect <session>
```

### Project Structure

```text
crates/agent-console-dashboard/
├── src/
│   ├── daemon/
│   │   └── protocol.rs   # IPC message parsing
│   └── client/
│       ├── mod.rs
│       └── commands.rs   # CLI client commands
```

### Key Dependencies

| Crate      | Purpose                         |
| ---------- | ------------------------------- |
| clap       | CLI argument parsing            |
| serde_json | JSON serialization for messages |
| tokio      | Async socket communication      |

### Design Decisions

- **JSON Lines format** — One JSON object per `\n`-delimited line. JSON
  serializers escape `\n` inside strings, so no framing ambiguity. See
  [D2](../architecture/2026-01-31-discussion-decisions.md).
- **Transport-independent** — JSON Lines is the wire format. Unix socket is the
  transport (Linux/macOS). Future Windows: named pipes. Future v1+: could swap
  to TCP or SQLite without changing wire format.
- **Push model for subscriptions** — Server pushes updates, clients don't poll
- **SUBSCRIBE semantics** — Sends full state snapshot first, then deltas on
  change
- **Protocol version** — Include `"version": 1` in messages for forward
  compatibility

### Complexity Review Notes

The [complexity review](../decisions/complexity-review.md) identified:

- `remove()` vs `remove_session()` — duplicate methods to consolidate
- `get_or_create_session()` — consider simplifying
- Full serde serialization may be premature for the current text protocol

Address these during implementation of S003.01 (protocol definition).
