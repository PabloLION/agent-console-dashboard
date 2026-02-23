# Hook Config Fields: `Vec<HookConfig>`

Created: 20260215T030000Z Issue: acd-1j2 Updated: 20260223T000000Z Issue:
acd-hgaz

## Evolution

### Phase 1 (acd-1j2): `Option<String>`

Hook fields started as `Option<String>` where `None` meant "not configured".
`None` was semantically cleaner than empty string as a sentinel.

### Phase 2 (acd-hgaz): `Vec` of HookConfig

Redesigned to match the Claude Code hook structure: an array of hook objects,
each with `command` and `timeout` fields. An empty vec means "not configured".

## Decision

Use a `Vec` of `HookConfig` for `activate_hooks` and `reopen_hooks`.

```rust
pub struct HookConfig {
    pub command: String,
    pub timeout: u64, // seconds, default 5
}
```

### Why array format

- Supports multiple hooks per event without needing shell composition
- Per-hook timeout is explicit and enforced by the runtime
- Matches the Claude Code hook structure (consistent mental model)
- TOML array-of-tables (`[[tui.activate_hooks]]`) is readable and unambiguous

### Why no backward compatibility

Users must update their config when upgrading. The old `activate_hook = "..."`
string format produces a TOML parse error, which is clear and actionable.
Migration code adds complexity for a breaking change that affects only hook
users who are actively configuring the TUI.

### Execution model

Hooks run sequentially. Each hook is spawned via `sh -c`, with session data in
env vars (`ACD_SESSION_ID`, `ACD_WORKING_DIR`, `ACD_STATUS`) and a JSON
`SessionSnapshot` on stdin. Stdout/stderr are captured and logged at debug
level. A hook exceeding its timeout is killed; the next hook still runs.
