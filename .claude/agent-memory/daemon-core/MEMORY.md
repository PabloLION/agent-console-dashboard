# Daemon Core Agent Memory

## Session Priority Implementation (acd-wx6, acd-nd1)

Added session priority system for sorting sessions in TUI:

### Key Changes

1. **Session struct** (`lib.rs`):
   - Added `priority: u64` field (default 0, higher = ranked higher)
   - Updated `Session::new()` and `Default for Session`

2. **Status enum** (`lib.rs`):
   - Added `status_group()` method returning u8 for sort ordering
   - Groups: Attention=0, Working=1, Question=2, Closed=3
   - Inactive sessions (not a Status variant) are treated as group 2 at sort
     time

3. **SessionSnapshot** (`ipc.rs`):
   - Added `priority: u64` field with `#[serde(default)]` for backwards compat
   - Updated `From<&Session>` conversion to include priority

4. **IpcCommand** (`ipc.rs`):
   - Added `priority: Option<u64>` field with
     `#[serde(skip_serializing_if = "Option::is_none")]`
   - All IpcCommand constructions updated to include `priority: None` or actual
     value

5. **SessionStore** (`daemon/store/`):
   - Renamed `broadcast_status_change` → `broadcast_session_change`
   - Updated to accept `old_priority` parameter
   - Broadcasts when status OR priority changes
   - `get_or_create_session` now accepts `priority: u64` parameter
   - Updates priority atomically alongside status

6. **TUI sorting** (`tui/app/update.rs`):
   - Sort applied in `apply_update()` after each session update
   - Sort key: `(status_group, Reverse(priority), Reverse(elapsed_seconds))`
   - Inactive detection: `session.is_inactive(INACTIVE_SESSION_THRESHOLD)`
   - Closed sessions always group 3 regardless of idle time

7. **CLI** (`main.rs`, `commands/ipc.rs`):
   - Added `--priority <u64>` optional flag to Set command
   - `run_set_command` now accepts `priority: Option<u64>`
   - Priority defaults to 0 if not provided

### Patterns

- **Option<PathBuf> for working_dir**: None when missing, no sentinel values
- **#[serde(default)] for new fields**: Backwards compatibility on wire format
- **#[serde(skip_serializing_if = "Option::is_none")]**: Omit optional fields
  from JSON
- **Atomic updates**: `get_or_create_session` updates status AND priority under
  single write lock
- **Broadcast on any change**: Fire notification if status OR priority changed

### Test Updates

All test files updated to pass priority parameter (default 0) to
`get_or_create_session`:

- `daemon/handlers/tests.rs`
- `daemon/store/tests/lifecycle_get_or_create.rs`
- Added sorting tests in `tui/app/tests/basic.rs`

### Files Modified

- `src/lib.rs` - Session struct, Status enum
- `src/ipc.rs` - SessionSnapshot, IpcCommand
- `src/daemon/store/mod.rs` - broadcast_session_change
- `src/daemon/store/lifecycle.rs` - get_or_create_session signature
- `src/daemon/store/closed.rs` - close_session broadcast call
- `src/daemon/handlers/mod.rs` - handle_set_command
- `src/tui/app/update.rs` - sorting logic
- `src/tui/app/tests/basic.rs` - sorting tests
- `src/main.rs` - CLI Set command
- `src/commands/ipc.rs` - run_set_command
- All IpcCommand construction sites

## Important Notes

- session_id is UUID v4 (36 chars), stable across resume/clear/compact
- INACTIVE_SESSION_THRESHOLD = 3600 seconds (1 hour)
- Tests must not hardcode version numbers — use `env!("CARGO_PKG_VERSION")`
- RwLock pattern (not Actor model) for shared state
- TOCTOU prevention: single atomic get_or_create_session
