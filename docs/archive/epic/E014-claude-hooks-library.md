# Epic: Claude Hooks Library

**Epic ID:** E014 **Status:** Draft **Priority:** High **Estimated Effort:** M

## Summary

Create a standalone Rust crate (`claude-hooks`) that enables programmatic management of Claude Code hooks through install, uninstall, and list operations. The library provides atomic file write safety, registry-based ownership tracking, and preserves user data integrity. Version 0.1 is library-only (no CLI), targeting user scope hooks exclusively. ACD daemon will consume this crate for lifecycle hook management.

## Goals

- Enable programmatic hook management without manual JSON editing
- Provide atomic write safety guarantees (no partial state, no data loss)
- Track hook ownership to distinguish ACD-managed vs user-created hooks
- Support install, uninstall, and list operations for user scope
- Zero-config operation (no setup files required)
- Deliver library-only API (CLI deferred to v0.2)

## User Value

Currently, ACD daemon cannot reliably install or remove hooks because manual JSON editing risks data corruption and has no ownership tracking. With claude-hooks:

- ACD can safely install Start, Stop, and BeforePrompt hooks during daemon startup
- ACD can cleanly remove only its hooks during shutdown (never touching user hooks)
- Developers get a reusable library for any Claude Code hook management needs
- All operations have atomic safety guarantees (write succeeds completely or fails completely)

## Stories

| Story ID | Title | Priority | Status |
| -------- | ----- | -------- | ------ |
| [S014.01](../stories/S014.01-scaffold-crate.md) | Scaffold claude-hooks crate | P0 | Draft |
| [S014.02](../stories/S014.02-types-and-errors.md) | Types and errors | P0 | Draft |
| [S014.03](../stories/S014.03-settings-reader-writer.md) | Settings reader/writer | P0 | Draft |
| [S014.04](../stories/S014.04-registry-reader-writer.md) | Registry reader/writer | P0 | Draft |
| [S014.05](../stories/S014.05-public-api.md) | Public API implementation | P0 | Draft |
| [S014.06](../stories/S014.06-tests.md) | Comprehensive tests | P0 | Draft |
| [S014.07](../stories/S014.07-wire-into-acd.md) | Wire into ACD daemon | P1 | Draft |

## Dependencies

- None (standalone crate)

## Related Epics

- [E001 - Daemon Core Infrastructure](./E001-daemon-core-infrastructure.md) - Daemon lifecycle management consumes this crate for hook installation/removal

## Acceptance Criteria

- [ ] Crate compiles and all tests pass (coverage >80%)
- [ ] `install()`, `uninstall()`, and `list()` functions work correctly
- [ ] Atomic writes guarantee no settings.json corruption
- [ ] Registry accurately tracks which hooks were installed by the crate
- [ ] All operations complete in <100ms (install/uninstall)
- [ ] Zero clippy warnings, zero type errors
- [ ] ACD daemon successfully uses library for all 3 hooks
- [ ] All workspace tests still pass (473+ tests)

## Technical Notes

### Scope

v0.1 limitations (by design):
- User scope only (`~/.claude/settings.json`)
- Library-only (no CLI binary)
- No multi-scope support
- No export/import
- No enable/disable toggles

Future versions (v0.2+): CLI binary, multi-scope, export/import.

### API Overview

```rust
use claude_hooks::{HookEvent, HookHandler, install, uninstall, list};

// Install hook
let handler = HookHandler {
    r#type: "command".to_string(),
    command: "/path/to/stop.sh $SESSION_ID".to_string(),
    matcher: String::new(),
    timeout: Some(600),
    r#async: None,
};
install(HookEvent::Stop, handler, "acd")?;

// Uninstall hook
uninstall(HookEvent::Stop, "/path/to/stop.sh $SESSION_ID")?;

// List all hooks
for entry in list()? {
    println!("{:?} - managed: {}", entry.event, entry.managed);
}
```

### Key Design Decisions

| ID | Decision | Impact |
|----|----------|--------|
| D01 | Atomic rename pattern | NFR1 (data integrity) |
| D16 | Local registry in XDG data dir | FR5 (ownership tracking) |
| D20 | Registry tracks only our hooks | FR2 (safe uninstall) |
| D22 | Hook identity is composite key | TC2 (identity definition) |
| D34 | Command format is full string | TC3 (command with args) |

See [design-draft.md](../../crates/claude-hooks/docs/design-draft.md) for all 36 decisions.

### File Locations

- **Claude settings:** `~/.claude/settings.json` (atomic writes with temp file)
- **Registry:** `$XDG_DATA_HOME/claude-hooks/registry.jsonc` (typically `~/.local/share/claude-hooks/registry.jsonc`)

### Atomic Write Strategy

1. Create temp file in same directory: `settings.json.tmp.20260203-143022`
2. Write JSON to temp file
3. Flush to disk (fsync)
4. Rename temp file to `settings.json` (atomic operation)
5. On error before rename: preserve temp file as "safety copy"

### Registry Schema

```jsonc
{
  "schema_version": 1,
  "agent_name": "claude-code",
  "hooks": [
    {
      // Identity (composite key)
      "event": "Stop",
      "matcher": "",
      "type": "command",
      "command": "/path/to/stop.sh",

      // Configuration
      "timeout": 600,
      "async": false,

      // Metadata
      "scope": "user",
      "enabled": true,
      "added_at": "20260203-143022",
      "installed_by": "acd",
      "description": "Sets session status to 'attention' on Stop event",
      "reason": "Notify ACD daemon when Claude Code stops",
      "optional": false
    }
  ]
}
```

### Dependencies

```toml
[dependencies]
thiserror = "1"                  # Error types
serde = { version = "1", features = ["derive"] }
serde_json = "1"                 # Claude settings.json parsing
json-comments = "0.2"           # JSONC parsing for registry
chrono = "0.4"                  # Timestamp generation
log = "0.4"                     # Logging
dirs = "5"                      # XDG directory resolution
```

### Platform Support

- **Supported:** macOS, Linux
- **Not supported in v0.1:** Windows (deferred to v0.2+)

## Out of Scope

- CLI binary (v0.2+)
- Multi-scope support: user/project/local (v0.2+)
- Export/import functions (v0.2+)
- Enable/disable toggles (v0.4+)
- Hook validation (caller's responsibility)
- Package management (not a package manager)
- Cross-agent compatibility (separate crates for other agents)

## Success Metrics

| Metric | Target |
| ------ | ------ |
| Install operation | <100ms |
| Uninstall operation | <100ms |
| List operation | <50ms |
| Test coverage | >80% |
| Data corruption rate | 0 |
| Unintended hook removal | 0 |
