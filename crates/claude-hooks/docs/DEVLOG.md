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

### 2026-02-05: Architecture Review — acd-ojb Analysis

Reviewed acd-ojb (P0) which bundled two changes:
(1) auto-stop idle threshold, (2) actor model refactor.

**Decision: Split into separate concerns.**

#### Auto-stop idle timer

Add `const AUTO_STOP_IDLE_SECS: u64 = 3600` as a hardcoded constant.
Configurability deferred to acd-jnd (P4).

#### Actor model refactor — deferred (acd-51l, P4)

Amendment 2 proposed replacing `tokio::spawn` per connection +
`Arc<RwLock<HashMap>>` with single-threaded actor + plain `HashMap`.

**Analysis of current RwLock approach:**

```text
Hook/TUI client ──→ tokio::spawn ──→ handle_client()
                                          │
                         .read().await or .write().await
                                          │
                                          ▼
                    SessionStore { Arc<RwLock<HashMap<String, Session>>> }
```

- Multiple readers proceed in parallel (RwLock allows concurrent reads)
- Writer waits for all readers, then gets exclusive access
- Arc enables shared ownership across spawned tasks

**Analysis of actor model alternative:**

```text
Hook/TUI client ──→ tokio::spawn ──→ mpsc::send(Command)
                                          │
                                     queue (FIFO)
                                          │
                                          ▼
                    Actor loop { owns HashMap directly, no locks }
```

- All operations (including reads) serialized through queue
- No locks, no deadlock possible, no TOCTOU races
- But: reads that could run in parallel now wait in line

**Tradeoff summary:**

| Concern | RwLock | Actor |
|---------|--------|-------|
| Memory safety | Rust guarantees | Same |
| Logic races (TOCTOU) | 1 theoretical case found | Impossible |
| Deadlocks | None found in actual code | Impossible |
| Code boilerplate | Less (idiomatic Rust) | More (message enums, oneshot channels) |
| Read throughput | Concurrent | Serialized |
| Refactor risk | None (working code) | High (~1600 lines) |

**Code audit results (2026-02-05):**

Attempted to construct concrete TOCTOU and deadlock examples from
the actual code:

- *TOCTOU:* One theoretical case in `handle_set_command` (server.rs:419-442).
  `get_or_create_session()` releases its lock before `update_session()`
  acquires a new one. A concurrent `RM` between the two calls could
  cause the "BUG" error on line 436. However: requires exact timing
  of SET+RM on the same session, consequence is just an error message
  (no data corruption), and is extremely unlikely at our scale.
- *Deadlocks:* Could not construct any. `close_session()` releases
  `sessions` lock (line 490) before acquiring `closed` lock (line 495).
  No method in the codebase holds two locks simultaneously. Lock
  nesting does not occur.

**Decision rationale:** The amendment's reasoning ("eliminates all race
conditions and RwLock complexity") overstated the risk for Rust code.
Rust's type system prevents data races at compile time. The one TOCTOU
found is inconsequential. No deadlocks are possible in the current code.
RwLock is idiomatic Rust — the language's ownership model is designed
for this pattern. The actor model would add boilerplate to solve problems
that don't exist in practice.

**Outcome:**

- acd-ojb closed (original issue bundling idle timer + actor refactor)
- acd-51l (P4): Actor model refactor deferred to backlog
- acd-jnd (P4): Idle timeout configurability deferred to backlog
- acd-2co (P1): Implement idle timer with hardcoded constant (3600s)
