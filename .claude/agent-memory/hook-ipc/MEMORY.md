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

### Env Vars Pattern (acd-ynba)

Hook env vars are set via `.env()` on `std::process::Command`:
- `ACD_SESSION_ID` — session.session_id
- `ACD_WORKING_DIR` — working_dir.display() or empty string
- `ACD_STATUS` — session.status.to_string()

Extract owned values BEFORE calling `session.into()` since `.into()` consumes
the `&Session` reference:
```rust
let session_id = session.session_id.clone();
let working_dir_str = session.working_dir...;
let status_str = session.status.to_string();
let snapshot: SessionSnapshot = session.into(); // session borrow ends here
```

### Documentation Location

Hook user documentation lives in `docs/user/`:

- Existing: `environment-variables.md`
- Updated: `double-click-hook.md` (removed placeholder section, added env vars)

## Files Modified (acd-7jh)

- `crates/agent-console-dashboard/src/tui/app/mod.rs`: Modified
  `execute_double_click_hook()` to pipe JSON to stdin
- `crates/agent-console-dashboard/src/tui/app/tests/interaction.rs`: Added
  `test_execute_double_click_hook_serializes_session_snapshot()`
- `docs/user/double-click-hook.md`: Created comprehensive hook documentation
  with JSON schema, examples, and field descriptions

## Files Modified (acd-ynba)

- `crates/agent-console-dashboard/src/tui/app/mod.rs`: Removed
  `substitute_hook_placeholders()`, added `.env()` calls in `execute_hook()`
- `crates/agent-console-dashboard/src/tui/app/tests/interaction.rs`: Removed 5
  `test_substitute_hook_*` tests
- `crates/agent-console-dashboard/src/config/default.rs`: Updated template
  comments — env vars instead of `{placeholders}`, added TOML escaping guidance
- `crates/agent-console-dashboard/src/config/schema.rs`: Updated doc comments
  and test strings to use env var syntax
- `docs/user/double-click-hook.md`: Full rewrite for env var approach
- `docs/configuration.md`: Updated `tui.double_click_hook` → `tui.activate_hook`
  / `tui.reopen_hook` sections with env var syntax
- `docs/decisions/resurrect-to-reopen.md`: Updated placeholder examples

## SessionSnapshot Re-export

Already complete - no changes needed:

- Defined in `src/ipc.rs` as public struct
- Re-exported from `src/lib.rs` via `pub use ipc::*;` (line 47)
- Available to hook authors who depend on `agent-console-dashboard` crate
