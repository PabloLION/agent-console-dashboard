# Research: Session Deletion Broadcast to TUI Clients

Issue: acd-wtr7

## Summary

**Simple addition.** Broadcasting session deletion events requires minimal
changes. The daemon already has broadcast infrastructure via
`tokio::sync::broadcast` channels. Adding a deletion event follows the exact
same pattern as existing notification types.

## Current Broadcast Mechanism

1. `SessionStore` contains `update_tx: broadcast::Sender<SessionUpdate>` for
   notifying subscribed TUI clients
2. `SessionStore::subscribe()` returns a receiver for clients
3. `broadcast_session_change()` sends notifications on status/priority changes
4. `IpcNotification` enum distinguishes message types: "update", "usage", "warn"

## What Exists

**Daemon side** (`src/daemon/`):

- `SessionStore::update_tx` broadcast channel
- `SessionUpdate` struct (session_id, status, elapsed_seconds)
- `IpcNotification` with constructors: `session_update()`, `usage_update()`,
  `warn()`
- `handle_sub_command()` forwards broadcast messages to TUI clients

**TUI side** (`src/tui/`):

- `DaemonMessage` enum with `SessionUpdate` and `UsageUpdate` variants
- `parse_daemon_line()` parses "update", "usage", "warn" types
- `App::apply_update()` handles updates (update existing or create new)

## What's Needed

4 files, ~50 lines total:

1. **`src/ipc.rs`**: Add `IpcNotification::session_delete(session_id)` constructor
   (serializes as `{"version": 1, "type": "delete", "session_id": "..."}`)
2. **`src/daemon/store/mod.rs`**: Broadcast deletion in `SessionStore::remove()`
   after removing from map
3. **`src/tui/subscription.rs`**: Add `SessionDelete(String)` to `DaemonMessage`,
   handle "delete" type in `parse_daemon_line()`
4. **`src/tui/app/update.rs`**: Handle `SessionDelete` by removing session from
   `self.sessions`

No changes needed in `handle_sub_command` — it already forwards all broadcast
messages.

## Assessment

Simple because:

- Existing broadcast channel and subscription mechanism work as-is
- New "delete" type follows the exact pattern of "update" and "usage"
- No new dependencies
- Backwards compatible (older TUI clients ignore unknown types)
- ~50 lines of code + tests, estimated <1 hour
