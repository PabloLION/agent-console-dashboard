# claude-hooks Development Log

Authoritative record of crate progress, decisions, and state.

## Crate Overview

`claude-hooks` — a Rust library for programmatic management of Claude Code hooks.
Reads/writes `~/.claude/settings.json` with atomic safety, tracks installed hooks
via local registry.

## Version Roadmap

| Version | Features                                              | Status       |
| ------- | ----------------------------------------------------- | ------------ |
| v0.1    | install, uninstall, list (user scope), registry       | **Complete** |
| v0.2    | Multi-scope, export/import, CLI binary                | Designed     |
| v1.0    | All v0.2 + doctor                                     | Designed     |
| Post-v1 | migrate, enable/disable, templates, cross-agent       | Deferred     |

## Current State (2026-02-04)

### v0.1.2 Implementation (Correct Format)

| Component | Files | Tests | Status |
| --------- | ----- | ----- | ------ |
| Types/Errors | `error.rs`, `types.rs` | 12 | ✅ |
| Settings I/O | `settings.rs` | 11 | ✅ |
| Registry I/O | `registry.rs` | 29 | ✅ |
| Public API | `lib.rs` | 11 | ✅ |
| Integration | `tests/*.rs` | 44 | ✅ |
| ACD Wiring | `daemon/mod.rs` | 12 | ✅ |

**Total: 89 tests passing**

### Documents

| Document           | Purpose                          | Status |
| ------------------ | -------------------------------- | ------ |
| `design-draft.md`  | 36 design decisions, formats     | Done   |
| `architecture.md`  | Module structure, data flow, API | Done   |
| `PRD.md`           | Product requirements v0.1        | Done   |

### What's Next

- v0.2: Multi-scope support, CLI binary, export/import

## Story Dependency Map

### Dependency Graph

```text
Layer 0:  S014.01 (Scaffold)
             │
Layer 1:  S014.02 (Types/Errors)
             │
        ┌────┴────┐
Layer 2: S014.03   S014.04    ← Can run in PARALLEL
        (Settings) (Registry)
        └────┬────┘
             │
Layer 3:  S014.05 (Public API)
             │
Layer 4:  S014.06 (Tests)
             │
Layer 5:  S014.07 (Wire into ACD)
```

### Story Details

| Story   | Title                | Points | Blocks     | Blocked By    | Parallel? |
| ------- | -------------------- | ------ | ---------- | ------------- | --------- |
| S014.01 | Scaffold Crate       | 1      | S014.02    | -             | No        |
| S014.02 | Types and Errors     | 2      | S014.03,04 | S014.01       | No        |
| S014.03 | Settings Reader      | 3      | S014.05    | S014.02       | **Yes**   |
| S014.04 | Registry Reader      | 2      | S014.05    | S014.02       | **Yes**   |
| S014.05 | Public API           | 3      | S014.06    | S014.03,04    | No        |
| S014.06 | Tests                | 3      | S014.07    | S014.05       | No        |
| S014.07 | Wire into ACD        | 2      | -          | S014.06       | No        |

### Parallelization Strategy

**Phase 1** (Sequential): S014.01 → S014.02
- Must be sequential: scaffold before types

**Phase 2** (Parallel): S014.03 + S014.04
- Both depend only on S014.02 (types/errors)
- No code dependency between them
- **Can spin up 2 agents in parallel**

**Phase 3** (Sequential): S014.05 → S014.06 → S014.07
- Each depends on previous
- Must be sequential

### Commit Checkpoints

| Checkpoint | After Story | Commit Message |
| ---------- | ----------- | -------------- |
| 1          | S014.01     | `feat: scaffold claude-hooks crate` |
| 2          | S014.05     | `feat: implement public API (install, uninstall, list)` |
| 3          | S014.06     | `test: add comprehensive test suite` |

### Agent Assignment Plan

```text
Agent 1: S014.01 (Scaffold)
         ↓
Agent 1: S014.02 (Types/Errors)
         ↓
         ├─→ Agent 1: S014.03 (Settings)
         └─→ Agent 2: S014.04 (Registry)  ← PARALLEL
         ↓
Agent 1: S014.05 (Public API) - after both complete
         ↓
Agent 1: S014.06 (Tests)
         ↓
Agent 1: S014.07 (Wire into ACD)
```

Total: 16 points, 7 stories, 3 commits, 1 parallel opportunity

## Key Design Decisions

| ID  | Decision                                            |
| --- | --------------------------------------------------- |
| D01 | Atomic rename pattern for settings.json writes      |
| D16 | Local registry in XDG data dir tracks our hooks     |
| D22 | Hook identity = composite key (event, matcher, type, command) |
| D36 | v0.1 is library-only (no CLI binary)                |

## Architecture Summary

```text
crates/claude-hooks/
├── Cargo.toml
├── docs/
│   ├── design-draft.md     # Design decisions
│   ├── architecture.md     # Technical architecture
│   ├── PRD.md              # Product requirements
│   └── DEVLOG.md           # This file
├── src/
│   ├── lib.rs              # Public API: install, uninstall, list
│   ├── error.rs            # Error types (thiserror)
│   ├── types.rs            # HookEvent, HookHandler, RegistryEntry
│   ├── settings.rs         # Atomic read/write for settings.json
│   └── registry.rs         # JSONC registry in XDG data dir
└── tests/
    ├── integration_tests.rs  # Full workflow tests
    ├── edge_cases.rs         # Sync issues, corrupt files
    ├── atomic_safety.rs      # Write safety, roundtrip
    └── performance.rs        # Timing targets (<100ms)
```

## Public API (v0.1.2)

```rust
pub fn install(event: HookEvent, handler: HookHandler, matcher: Option<String>, installed_by: &str) -> Result<()>;
pub fn uninstall(event: HookEvent, command: &str) -> Result<()>;
pub fn list() -> Result<Vec<ListEntry>>;
```

## Testing Patterns

Reusable patterns for testing file-based operations.

### HOME Isolation Pattern

Tests that read/write user-scoped files (`~/.claude/settings.json`, XDG data dirs) should
isolate themselves by setting HOME to a temp directory:

```rust
use tempfile::tempdir;
use std::env;

fn setup_test_env() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    env::set_var("HOME", dir.path());

    // Create required directories
    std::fs::create_dir_all(dir.path().join(".claude")).unwrap();

    // Create minimal settings.json
    let settings = r#"{"hooks": {}}"#;
    std::fs::write(dir.path().join(".claude/settings.json"), settings).unwrap();

    dir  // Return TempDir to keep it alive for test duration
}

#[test]
fn test_example() {
    let _dir = setup_test_env();  // Keep TempDir alive
    // Test code uses isolated HOME
}
```

**Benefits:**

- No interference with real user settings
- Tests can run in parallel safely
- Automatic cleanup when TempDir drops
- Deterministic initial state

**Caveats:**

- Must keep TempDir in scope for test duration
- Use `#[serial]` from `serial_test` crate if tests must run sequentially
- XDG dirs (`dirs::data_dir()`) also respect HOME changes

Apply this pattern in all tests that touch `settings.json` or registry files.

## Session History

### 2026-02-03: Initial Design

- Created design-draft.md with 36 decisions
- Created architecture.md with module specs
- Established v0.1 scope (library-only, user scope)
- Selected dependencies consistent with workspace patterns

### 2026-02-04: v0.1 Implementation Complete

**Phase 1 (Sequential):**
- S014.01 (Scaffold): Crate created, added to workspace
- S014.02 (Types/Errors): 12 unit tests, HookEvent enum, error hierarchy

**Phase 2 (Parallel):**
- S014.03 (Settings): 11 tests, atomic write with temp-file-then-rename
- S014.04 (Registry): 29 tests, JSONC parsing, XDG path resolution
- Both agents ran simultaneously — no conflicts

**Phase 3 (Sequential):**
- S014.05 (Public API): install/uninstall/list wired together, 11 integration tests
- S014.06 (Comprehensive Tests): 44 tests in `tests/` directory
  - `integration_tests.rs`: Full workflows
  - `edge_cases.rs`: Corrupt files, sync issues, missing data
  - `atomic_safety.rs`: Roundtrip preservation, large files
  - `performance.rs`: All operations <100ms
- S014.07 (Wire into ACD): Daemon startup/shutdown hooks, crash recovery

**Commit:** `6d8fb5f` feat: add claude-hooks library for programmatic hook management

**Stats:**
- 25 files changed, 8691 insertions
- 83+ tests passing
- Epic E014 complete (16 story points)

### 2026-02-04: Fix Hooks Format (v0.1.1 → v0.1.2)

**Problem:** v0.1.0 and v0.1.1 used an incorrect array-based hooks format.
Claude Code actually uses an object-based format:

```json
{
  "hooks": {
    "EventName": [
      { "matcher": "optional_regex", "hooks": [{ "type": "command", "command": "..." }] }
    ]
  }
}
```

**Changes:**

- **types.rs:** Added `MatcherGroup` struct (intermediate level with optional matcher and hooks array). Moved `matcher` from `HookHandler` to group level. Added `status_message` field to `HookHandler`. Changed `RegistryEntry.matcher` from `String` to `Option<String>`.
- **settings.rs:** Rewrote `add_hook()`, `remove_hook()`, `list_hooks()` for object-based format. `add_hook()` now takes optional matcher parameter. `list_hooks()` uses resilient design — skips malformed entries instead of erroring.
- **lib.rs:** Updated `install()` to take 4 parameters: `(event, handler, matcher, installed_by)`.
- **error.rs:** Fixed `HookEvent::Start` → `HookEvent::SessionStart` in tests.
- **registry.rs:** Fixed `matcher: String::new()` → `matcher: None` in tests.
- **All 5 test files:** Updated to object-based format, new API signatures, correct event names, consistent `#[serial(home)]` group.

**Test results:** 89 tests passing (45 unit + 9 atomic + 16 edge + 9 integration + 10 performance)

**Commits:**
- `4bc351c` fix: use correct Claude Code hooks format (object-based)
- `04112e2` fix: update ACD daemon to use correct claude-hooks API
- `10e96a0` chore: bump claude-hooks to v0.1.2

**Published:** v0.1.2 on crates.io

**Remaining:**
- acd-vnd: Fix dead code warnings
- acd-9pq: Verify hook scripts exist at referenced paths
