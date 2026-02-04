# Product Requirements Document: claude-hooks v0.1

## Problem Statement

Agent Console Dashboard (ACD) needs to programmatically install, track, and remove Claude Code hooks during daemon lifecycle management. Currently, no library exists to safely manipulate Claude's settings.json while preserving user data integrity.

### User Pain Points

1. **Manual hook installation is error-prone** - Users must hand-edit JSON, risking syntax errors or data corruption
2. **No tracking of installed hooks** - ACD cannot distinguish its hooks from user-created hooks, making safe uninstall impossible
3. **No atomic write safety** - Direct JSON editing can corrupt settings.json if interrupted mid-write
4. **No programmatic access** - ACD daemon must shell out to text editors or use brittle string manipulation

### Business Impact

Without this library, ACD cannot:
- Reliably install hooks during daemon startup
- Safely remove hooks during daemon shutdown
- Verify hook installation state
- Distinguish managed vs unmanaged hooks

## Goals and Non-Goals

### Goals (v0.1)

**Primary Goal:** Enable ACD daemon to programmatically manage its 3 hooks (Start, Stop, BeforePrompt) with atomic safety guarantees.

**Specific Goals:**
1. Install hooks to `~/.claude/settings.json` without corrupting existing data
2. Track which hooks were installed by ACD (vs user-created hooks)
3. Uninstall only ACD-managed hooks (never touch user hooks)
4. List all hooks with managed/unmanaged indicators
5. Atomic file writes with safety copies on failure
6. Zero-config operation (no setup required)

### Non-Goals (v0.1)

**Out of scope for this release:**
1. **CLI binary** - Library-only in v0.1 (CLI ships in v0.2)
2. **Multi-scope support** - User scope only (project/local scopes in v0.2)
3. **Export/import** - Format designed but not implemented (v0.3)
4. **Enable/disable toggles** - Use uninstall/reinstall instead (v0.4)
5. **Hook validation** - Caller responsible for ensuring hooks work
6. **Package management** - We are a settings.json editor, not npm for hooks
7. **Cross-agent compatibility** - Claude Code only (other agents get separate crates)

### Success Criteria

**Must achieve:**
1. Zero data loss - Never corrupt settings.json or lose existing hooks
2. Atomic operations - All writes succeed completely or fail completely
3. Ownership tracking - 100% accuracy distinguishing managed vs unmanaged hooks
4. ACD integration - Daemon successfully uses library for all 3 hooks

**Performance targets:**
- Install operation: <100ms
- Uninstall operation: <100ms
- List operation: <50ms

## User Personas

### Primary: ACD Daemon (Programmatic Caller)

**Who:** Rust program that manages Claude Code lifecycle

**Needs:**
- Install 3 hooks on daemon startup (Start, Stop, BeforePrompt)
- Verify hooks are installed correctly
- Uninstall hooks on daemon shutdown
- Distinguish ACD hooks from user-created hooks
- Atomic safety guarantees (no partial writes)

**Usage pattern:**
```rust
use claude_hooks::{HookEvent, HookHandler, install, uninstall, list};

// On daemon startup
let handler = HookHandler {
    r#type: "command".to_string(),
    command: "/path/to/acd/hooks/stop.sh $SESSION_ID".to_string(),
    matcher: String::new(),
    timeout: Some(600),
    r#async: None,
};
install(HookEvent::Stop, handler, "acd")?;

// On daemon shutdown
uninstall(HookEvent::Stop, "/path/to/acd/hooks/stop.sh $SESSION_ID")?;
```

### Secondary: Future CLI Users (v0.2+)

**Who:** Users who want to manage hooks manually via command line

**Status:** Not supported in v0.1. CLI binary ships in v0.2.

### Tertiary: Advanced Users

**Who:** Users who manually edit settings.json for custom hooks

**Interaction:** Library must coexist peacefully with manual edits. List command shows both managed and unmanaged hooks. Uninstall never touches hooks we didn't install.

## Requirements

### Functional Requirements

#### FR1: Install Hook (Priority: P0)

**What:** Add a hook to `~/.claude/settings.json` and register it in local registry.

**Inputs:**
- `event: HookEvent` - One of 12 hook events (Start, Stop, BeforePrompt, etc.)
- `handler: HookHandler` - Hook configuration (type, command, matcher, timeout, async)
- `installed_by: &str` - Free-form installer identifier (e.g., "acd")

**Behavior:**
1. Check if hook already exists in registry → error if duplicate
2. Check if hook already exists in settings.json → error if duplicate
3. Parse settings.json preserving all non-hook data
4. Add hook to hooks array
5. Write settings.json atomically (temp file + rename)
6. Add entry to registry with metadata (added_at timestamp, installer, etc.)

**Error conditions:**
- Hook already exists (in registry or settings)
- Settings.json not found or unreadable
- Settings.json parse error
- Atomic write failure (return temp file path for recovery)
- Registry write failure (log warning, continue - hook installed but untracked)

**Design references:** D01 (atomic writes), D16 (registry tracking), D22 (composite key identity)

#### FR2: Uninstall Hook (Priority: P0)

**What:** Remove a hook from `~/.claude/settings.json` and registry, only if installed by this crate.

**Inputs:**
- `event: HookEvent` - Hook event
- `command: &str` - Exact command string

**Behavior:**
1. Check registry for matching entry → error if not found (hook not managed)
2. Parse settings.json
3. Remove hook from hooks array (exact match on event + command)
4. Write settings.json atomically
5. Remove entry from registry
6. If hook in registry but not in settings → log warning, remove from registry anyway

**Error conditions:**
- Hook not managed by this crate (not in registry)
- Settings.json not found or unreadable
- Atomic write failure

**Safety invariant:** Never remove hooks we didn't install. Registry is source of truth for ownership.

**Design references:** D16 (registry ownership), D20 (track only our hooks)

#### FR3: List Hooks (Priority: P0)

**What:** Show all hooks from settings.json with managed/unmanaged status.

**Inputs:** None

**Outputs:** `Vec<ListEntry>` containing:
- `event: HookEvent` - Hook event
- `handler: HookHandler` - Hook configuration
- `managed: bool` - True if installed by this crate
- `metadata: Option<RegistryMetadata>` - Present if managed (added_at, installed_by, description, etc.)

**Behavior:**
1. Read registry
2. Read settings.json
3. Parse all hooks from settings.json
4. For each hook, check if exists in registry
5. Return list with managed flag and metadata where applicable

**Error conditions:**
- Settings.json not found or unreadable
- Registry not found (treat as empty)

**Design references:** D21 (show all hooks with ownership markers)

#### FR4: Atomic File Writes (Priority: P0)

**What:** Write settings.json with atomic safety guarantees.

**Strategy (D01):**
1. Create temp file in same directory: `settings.json.tmp.20260202-143022`
2. Write JSON to temp file
3. Flush to disk (fsync)
4. Rename temp file to `settings.json` (atomic operation)
5. On error before rename: preserve temp file and log path as "safety copy"

**Guarantees:**
- All-or-nothing write (never partial state)
- Original file untouched on failure
- Recovery path available (temp file preserved)

**Timestamp format (D03):** `yyyyMMdd-hhmmss` (17 chars with dash)

#### FR5: Registry Management (Priority: P0)

**What:** Track installed hooks in local registry file.

**Location (D16):** `$XDG_DATA_HOME/claude-hooks/registry.jsonc`
- Typical: `~/.local/share/claude-hooks/registry.jsonc`
- Create directory if missing

**Format (D19):** JSONC (JSON with comments)

**Schema (v1):**
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
      "added_at": "20260202-143022",
      "installed_by": "acd",
      "description": "Sets session status to 'attention' on Stop event",
      "reason": "Notify ACD daemon when Claude Code stops",
      "optional": false
    }
  ]
}
```

**Fields:**
- **Required:** event, matcher, type, command, scope, enabled, added_at, installed_by
- **Optional:** timeout, async, description, reason, optional

**Behavior:**
- Create on first install if missing
- Atomic writes same as settings.json
- Schema version for future migration

**Design references:** D16 (registry location), D19 (JSONC format), D20 (track only ours)

### Non-Functional Requirements

#### NFR1: Data Integrity (Priority: P0)

**Requirement:** Never corrupt settings.json under any circumstance.

**Implementation:**
- Atomic rename pattern (write temp + rename)
- Preserve all non-hook keys in settings.json
- Parse settings.json as `serde_json::Value` to retain unknown fields
- Safety copy on write failure (temp file preserved for recovery)

**Verification:**
- Unit tests: settings roundtrip preserves all keys
- Integration tests: interrupt simulation (permission denied, disk full)
- Manual testing: install/uninstall with real `~/.claude/settings.json`

#### NFR2: Performance (Priority: P1)

**Requirements:**
- Install: <100ms end-to-end
- Uninstall: <100ms end-to-end
- List: <50ms end-to-end

**Assumptions:**
- Settings.json <100KB
- Registry <10KB
- Hooks array <50 entries

**Trade-offs:** Simplicity over optimization. No caching, no incremental updates. Read entire file, modify, write entire file. Acceptable for expected data sizes.

#### NFR3: Usability (Priority: P1)

**Requirements:**
- Zero-config operation (no setup files)
- Clear error messages with context (file paths, hook identity)
- Logging at appropriate levels (debug/warn/error)
- Absolute file paths in all outputs

**Error message examples:**
- "Hook already exists: Stop - /path/to/stop.sh"
- "Failed to write settings atomically: /Users/pablo/.claude/settings.json - Safety copy at: /Users/pablo/.claude/settings.json.tmp.20260202-143022"
- "Hook not managed by claude-hooks: BeforePrompt - /custom/hook.sh"

#### NFR4: Platform Support (Priority: P1)

**Supported:**
- macOS (primary development target)
- Linux (XDG conventions)

**Not supported in v0.1:**
- Windows (XDG conventions differ, deferred to v0.2+)

#### NFR5: Code Quality (Priority: P1)

**Standards:**
- Follow existing workspace patterns (same as `claude-usage` crate)
- Use `thiserror` for error types
- Use `serde_json` for JSON parsing
- Use `log` crate for logging
- All public functions have doc comments with examples
- All error variants have clear error messages

**Testing:**
- Unit tests for all pure functions
- Integration tests for install/uninstall/list flows
- Edge case tests (registry write fails, corrupt settings.json, etc.)
- Test coverage >80%

### Technical Constraints

#### TC1: Settings.json Structure (D13)

**Constraint:** Must preserve Claude Code's settings.json structure.

**Top-level keys to preserve:**
- `cleanupPeriodDays`
- `env`
- `permissions`
- `hooks` (array, modified by us)
- `statusLine`
- `enabledPlugins`
- `syntaxHighlightingDisabled`

**Approach:** Parse as `serde_json::Value`, modify only `hooks` array, write back.

#### TC2: Hook Identity (D22)

**Constraint:** Hook identity is a composite key, not a hash or ID.

**Identity fields:**
- `event` (HookEvent enum)
- `matcher` (string)
- `type` (string, always "command" in v0.1)
- `command` (full command string with args)

**Non-identity fields (configuration):**
- `timeout`
- `async`

**Implication:** Two hooks with same command but different timeouts are considered the same hook (identity match). Only one can exist.

#### TC3: Command Format (D34)

**Constraint:** `command` field is a full shell command string, not just a file path.

**Examples:**
- `/path/to/stop.sh $SESSION_ID $ARGS`
- `python3 /path/to/script.py --verbose`
- `/usr/bin/env node /path/to/hook.js --flag`

**Identity matching:** Exact string comparison. Whitespace matters.

#### TC4: User Scope Only (D12, D25)

**Constraint:** v0.1 only supports user scope.

**Location:** `~/.claude/settings.json`

**Future scopes (v0.2+):**
- Project: `.claude/settings.json`
- Local: `.claude/settings.local.json`

**Implication:** No scope parameter in v0.1 API. Scope hardcoded to "user" in registry entries.

### Dependencies

#### External Dependencies

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

**Justifications:**
- **json-comments** - Simplest JSONC parser (O01 resolved)
- **chrono** - Standard for timestamps (existing workspace dependency)
- **dirs** - Cross-platform XDG directory resolution
- **thiserror** - Follows `claude-usage` pattern

#### Internal Dependencies

**Caller:** `agent-console-dashboard` crate

**Integration point:** ACD daemon lifecycle management (start/stop hooks)

## Success Metrics

### Launch Criteria (v0.1 Release)

**Must have:**
1. All 3 public functions implemented and tested
2. Integration with ACD daemon complete
3. Zero regressions in ACD test suite (473+ tests still pass)
4. Manual testing completed (install/uninstall/list with real settings.json)
5. Documentation complete (README, doc comments, examples)

**Quality gates:**
1. All tests pass (`cargo test -p claude-hooks`)
2. Clippy warnings = 0 (`cargo clippy -p claude-hooks`)
3. Type errors = 0 (`cargo check -p claude-hooks`)
4. Test coverage >80%

### Post-Launch Metrics

**Adoption:**
- ACD daemon successfully uses library for all 3 hooks
- Zero data corruption reports (settings.json)
- Zero unintended hook removal reports (ownership tracking)

**Reliability:**
- Zero panics in production use
- All errors handled gracefully (no unwrap/expect in production code)
- Safety copy mechanism never needed (atomic writes succeed)

**Usability:**
- Clear error messages in all failure scenarios
- Logs actionable for debugging

## Risks and Mitigations

### R1: Settings.json Corruption (High Impact, Medium Probability)

**Risk:** Atomic write interrupted → settings.json lost or corrupted.

**Mitigations:**
- Atomic rename pattern (write temp, rename over original)
- Safety copy preserved on failure (temp file not deleted)
- Error message includes temp file path for recovery
- Integration tests simulate write failures

**Residual risk:** Filesystem bugs, power loss during rename (OS-level issue, out of our control)

### R2: Registry Out of Sync (Medium Impact, Low Probability)

**Risk:** Registry write fails after settings write → hook installed but untracked.

**Mitigations:**
- Log warning (don't fail operation)
- List command shows hook as unmanaged
- User can uninstall via CLI in v0.2 (not automated in v0.1)

**Residual risk:** Hook appears unmanaged until user manually removes from settings.json or deletes registry file to reset.

### R3: Unknown Fields in settings.json (Low Impact, Unknown Probability)

**Risk:** Claude Code adds new fields to settings.json → our library doesn't preserve them.

**Mitigations:**
- Parse as `serde_json::Value` (preserves all fields)
- Only modify `hooks` array
- Write back entire structure

**Verification:** Integration test with extra top-level keys (simulate future Claude Code versions)

### R4: Metadata Fields Rejected (Low Impact, Unknown Probability)

**Risk:** Claude Code has JSON schema validation → rejects unknown fields in hook entries.

**Mitigations (O06):**
- Metadata only stored in registry (not settings.json)
- Hook entries in settings.json match Claude's exact structure
- No custom fields added to hooks array

**Status:** Design decision deferred. Test before v0.2. v0.1 safe (no metadata in settings.json).

### R5: Hook Deduplication Across Scopes (Low Impact, Unknown Probability)

**Risk:** Same hook exists in multiple scopes (user + project) → Claude Code behavior unknown.

**Status (O03):** Research needed before v0.2. Not a v0.1 issue (user scope only).

**Mitigation:** Document behavior once tested.

## Implementation Plan

### Phase 1: Scaffold (1 commit)

**Tasks:**
- Create `Cargo.toml` with author `Pablo LION <36828324+PabloLION@users.noreply.github.com>`
- Add `crates/claude-hooks` to workspace members
- Create `src/lib.rs` stub
- Verify: `cargo build -p claude-hooks`

### Phase 2: Types and Errors (1 commit)

**Tasks:**
- Implement `error.rs`: Error hierarchy with thiserror
- Implement `types.rs`: HookEvent, HookHandler, RegistryEntry, ListEntry
- Unit tests: serialization roundtrip, event string mapping

### Phase 3: Settings I/O (1 commit)

**Tasks:**
- Implement `settings.rs`: read/write with atomic safety
- Unit tests: roundtrip, preserve non-hook keys, atomic write failure

### Phase 4: Registry I/O (1 commit)

**Tasks:**
- Implement `registry.rs`: JSONC read/write in XDG data dir
- Unit tests: roundtrip, directory creation

### Phase 5: Public API (1 commit)

**Tasks:**
- Implement `lib.rs`: install, uninstall, list
- Integration tests: install → list → uninstall flows
- Edge case tests: registry out of sync, unmanaged hooks

### Phase 6: ACD Integration (1 commit)

**Tasks:**
- Add dependency to `agent-console-dashboard/Cargo.toml`
- Wire install/uninstall into daemon lifecycle
- Verify workspace builds and all tests pass

### Phase 7: Documentation (1 commit)

**Tasks:**
- README with usage examples
- Doc comments for all public functions
- Update DEVLOG with implementation status

**Total estimated commits:** 7

**Target timeline:** 1-2 days

## Out of Scope (Future Versions)

### v0.2 Features

- Multi-scope support (user/project/local)
- Export/import functions (agent representation format)
- CLI binary (`claude-hooks` command)

### v1.0 Features

- Doctor command (diagnose sync issues)
- All v0.2 features

### Post-v1 Features

- Migrate between scopes
- Enable/disable toggles
- Universal representation format
- Cross-agent compatibility (separate crates)
- Hook templates/catalog (different product)

## Appendix: Design Decisions Reference

| ID | Decision | Relevance |
|----|----------|-----------|
| D01 | Atomic rename pattern | NFR1 (data integrity) |
| D03 | Timestamp format yyyyMMdd-hhmmss | FR4 (atomic writes) |
| D12 | v0.1 scope: library-only, user scope | TC4 (scope constraint) |
| D13 | settings.json structure | TC1 (preserve structure) |
| D16 | Local registry in XDG data dir | FR5 (registry management) |
| D19 | Registry format: JSONC | FR5 (registry format) |
| D20 | Registry tracks only our hooks | FR2 (uninstall safety) |
| D21 | List shows all hooks with ownership markers | FR3 (list behavior) |
| D22 | Hook identity is composite key | TC2 (identity definition) |
| D24 | installed_by is free-form string | FR1 (install inputs) |
| D25 | Scope naming: user/project/local | TC4 (scope names) |
| D34 | Command format is full string | TC3 (command constraint) |
| D36 | v0.1 is library-only | Non-goal (no CLI) |

## Appendix: Open Questions

| ID | Question | Status | Impact |
|----|----------|--------|--------|
| O01 | JSONC parser choice | Resolved (json-comments) | None |
| O03 | Hook deduplication across scopes | Research needed | v0.2 blocking |
| O06 | Metadata fields in settings.json | Test before v0.2 | v0.2 design decision |

## Revision History

| Date | Version | Changes |
|------|---------|---------|
| 2026-02-03 | 1.0 | Initial PRD for v0.1 |
