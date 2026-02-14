# Hook IPC Agent Memory

## Key Patterns Learned

### SessionSnapshot JSON on stdin

Implemented hook stdin piping pattern for double-click hook in acd-7jh:

1. **Conversion**: Session → SessionSnapshot via `From<&Session>` impl (already
   existed in `src/ipc.rs` lines 131-171)
2. **Serialization**: `serde_json::to_string(&snapshot)`
3. **Piping**: Change `stdin(Stdio::null())` to `stdin(Stdio::piped())`, then
   `child.stdin.write_all(json_payload.as_bytes())`
4. **Fire-and-forget**: No need to wait for child process or handle errors after
   write

### Agent Type Serialization

`agent_type` field in SessionSnapshot uses
`format!("{:?}", agent_type).to_lowercase()`:

- `AgentType::ClaudeCode` → `"claudecode"` (not `"claude-code"`)
- Pattern defined at `src/ipc.rs:162`

### Testing Hook JSON

Test pattern for verifying SessionSnapshot serialization:

```rust
let session = Session::new(...);
let snapshot: SessionSnapshot = (&session).into();
let json_str = serde_json::to_string(&snapshot).unwrap();
let parsed: SessionSnapshot = serde_json::from_str(&json_str).unwrap();
assert_eq!(parsed.session_id, ...);
```

Don't spawn actual child processes in tests - they can hang.

### Documentation Location

Hook user documentation lives in `docs/user/`:

- Existing: `environment-variables.md`
- Added: `double-click-hook.md`

## Files Modified (acd-7jh)

- `crates/agent-console-dashboard/src/tui/app/mod.rs`: Modified
  `execute_double_click_hook()` to pipe JSON to stdin
- `crates/agent-console-dashboard/src/tui/app/tests/interaction.rs`: Added
  `test_execute_double_click_hook_serializes_session_snapshot()`
- `docs/user/double-click-hook.md`: Created comprehensive hook documentation
  with JSON schema, examples, and field descriptions

## SessionSnapshot Re-export

Already complete - no changes needed:

- Defined in `src/ipc.rs` as public struct
- Re-exported from `src/lib.rs` via `pub use ipc::*;` (line 47)
- Available to hook authors who depend on `agent-console-dashboard` crate
