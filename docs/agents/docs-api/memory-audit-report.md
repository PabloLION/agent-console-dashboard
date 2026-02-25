# Agent Memory Audit Report

**Date**: 20260224 **Auditor**: docs-api agent **Reference**:
`docs/decisions/INDEX.md` (40 decision documents)

This report compares each agent's persistent memory against the decisions index,
flagging project-specific design decisions that are missing from
`docs/decisions/` (gaps) and memory entries that contradict existing decision
documents (collisions).

Scope: only "we chose X over Y because Z" decisions. Implementation recipes,
file change lists, common Rust conventions, and test patterns are excluded.

---

## Agent: build-tooling

### Gaps (decisions not in docs/)

1. **Rust toolchain pinned to 1.93.1**: The project pins to a specific Rust
   version (`1.93.1`) via `rust-toolchain.toml` to ensure identical `rustfmt`
   output locally and in CI. The components list (`rustfmt`, `clippy`) is also
   fixed. Neither `implementation-defaults.md` (which lists "latest stable Rust
   only" under MSRV) nor any other decision doc records the pinning decision or
   the chosen version. (memory line ~8)

2. **Pre-commit test output filtering via shell grep**: The pre-commit hook
   captures `cargo test` output to a temp file and filters lines starting with
   dots, preserving headers and summaries only. The rationale (test harness
   offers only terse/pretty/JSON formats; shell-level filtering is the only
   option for quieter output) is a project-specific decision not recorded
   anywhere in `docs/decisions/`. (memory line ~42) _Note_: this is borderline —
   it could be seen as implementation detail rather than architectural decision.
   Flagged because the rationale ("only option") is a deliberate constraint
   worth preserving.

### Collisions (contradicts existing doc)

1. **MSRV policy mismatch**: `implementation-defaults.md` (Q53) states "Latest
   stable Rust only. New project with no legacy users." The build-tooling memory
   records pinning to Rust **1.93.1** — a specific, non-latest version. These
   directly contradict. The decision doc should be updated to reflect that the
   project pins to a specific version for CI reproducibility. (memory line ~10)

---

## Agent: cli-features

### Gaps (decisions not in docs/)

1. **`acd daemon restart` stops with force, starts if not running**: The restart
   command reuses `run_daemon_stop_command` with `force=true` (skips
   confirmation), then starts even if the daemon was not running. This is a
   design choice about restart semantics — "restart" means "ensure running with
   fresh env vars" rather than "fail if not already running." Not recorded in
   any decision doc. (memory line ~21)

2. **Uninstall command preserves config file**: `acd uninstall` stops the
   daemon, removes the socket, removes hooks — but deliberately does NOT delete
   the config file, instead printing the path for the user. This is an explicit
   design choice (user data preservation) not captured anywhere in
   `docs/decisions/`. (memory line ~121)

3. **Lazy-create is hooks-only; CLI errors on nonexistent sessions**: The
   `session-update-command.md` decision mentions this as an edge case, but
   cli-features memory records it as a governing rule: CLI commands against
   sessions require the session to exist. Hooks use `get_or_create_session`.
   `session-update-command.md` covers the update command specifically but does
   not document this as a general CLI vs hooks distinction. Possible gap
   (uncertain) — may be adequately implied by existing docs. (memory line ~106)

### Collisions (contradicts existing doc)

No collisions found.

---

## Agent: daemon-core

### Gaps (decisions not in docs/)

1. **Broadcast fires on status OR priority change**: `broadcast_session_change`
   triggers whenever either status or priority changes, not only on status
   changes. This is distinct from the original design (status-only broadcasts).
   `session-sorting.md` mentions the function name and that it handles priority,
   but does not document the broadcast trigger rule as a design decision.
   (memory line ~53)

2. **Short session ID in log messages (8 chars)**: Log messages truncate session
   IDs to 8 characters using `&id[..id.len().min(8)]`. This is a deliberate
   choice (safe for any length, UUID v4 is all ASCII so byte slicing is
   correct). No decision doc covers logging format choices. (memory line ~85)
   _Note_: borderline — implementation detail rather than architectural
   decision. Flagged because the safety reasoning ("byte slicing is correct for
   UUID v4") is worth preserving.

### Collisions (contradicts existing doc)

No collisions found. The `INACTIVE_SESSION_THRESHOLD = 3600` value in memory
matches `session-sorting.md`. Priority type `u64` with default 0 matches
`session-sorting.md`. RwLock preference is consistent with
`concurrency-model.md`.

---

## Agent: docs-api

No issues found.

The memory contains only documentation format conventions (CSV tables for env
vars, rustdoc `///` style) and session-specific task completion notes. No
project-specific design decisions are recorded that would need to be in
`docs/decisions/`.

---

## Agent: hook-ipc

### Gaps (decisions not in docs/)

1. **`AgentType` serializes as lowercase Debug string**: `AgentType::ClaudeCode`
   serializes to `"claudecode"` (not `"claude-code"` or `"ClaudeCode"`) via
   `format!("{:?}", agent_type).to_lowercase()`. This is a wire-format decision
   with interoperability consequences — hook authors who parse the `agent_type`
   field in the JSON `SessionSnapshot` depend on this value. Not mentioned in
   `ipc-protocol.md` or any other decision doc. (memory line ~20)

2. **Hook subprocess uses polling `try_wait()` every 50ms instead of
   `wait_with_output()`**: The execution model for activate/reopen hooks uses
   `try_wait()` polling in a background thread to enforce per-hook timeouts.
   `wait_with_output()` was explicitly rejected because it double-waits after
   `try_wait()`. `hook-field-type.md` covers the timeout concept (5s default,
   per-hook) but does not document the implementation choice between polling and
   blocking wait — which matters for future maintainers changing the execution
   model. (memory line ~133)

3. **TUI hook subprocess is fire-and-forget (no error propagation to TUI)**:
   When the TUI executes an activate or reopen hook, it runs in a background
   thread with no result reported back to the TUI (output is logged at debug
   level only). This is a design choice about error visibility.
   `hook-contract.md` covers the Claude Code side (exit codes, non-blocking
   errors to dashboards), but does not cover the TUI side's fire-and-forget
   execution model. (memory line ~15)

### Collisions (contradicts existing doc)

No collisions found.

---

## Agent: test-writer

No issues found.

The memory contains test organization patterns (TestBackend, disambiguation
rendering tests, env var elimination), version reference rules, and env var
elimination patterns. These are all implementation recipes or conventions
already covered by `testing-strategy.md`. No project-specific design decisions
are missing from `docs/decisions/`.

---

## Agent: tui-rendering

### Gaps (decisions not in docs/)

1. **Detail panel is always visible (12-line fixed section)**: The detail panel
   is always present in the layout — it does not toggle. When nothing is
   selected, it shows hint text. This is a deliberate UX decision (no show/hide
   toggle, placeholder over empty space) not captured in any decision doc.
   (memory line ~1)

2. **Scroll wheel navigates sessions, never scrolls detail panel**: Mouse scroll
   is reserved for session list navigation regardless of cursor position. The
   decision to never route scroll to the history panel (even when focused) is an
   explicit interaction model choice not documented anywhere. (memory line ~16)

3. **Enter key fires hook (same behavior as double-click)**: The Enter key
   executes the activate or reopen hook, not a detail-open action. This
   unification of keyboard and mouse activation is a design decision. Not
   recorded in any decision doc (resurrect-to-reopen.md mentions
   double-click/Enter/r for reopen but does not establish Enter as the canonical
   activation key for the general case). (memory line ~15)

4. **Esc key and header click clear selection (defocus)**: Pressing Esc or
   clicking the header sets `selected_index = None`. This defines what
   "deselect" means and what gestures trigger it. Not documented in any decision
   doc. (memory line ~17)

### Collisions (contradicts existing doc)

No collisions found.

---

## Agent: tui-visual

### Gaps (decisions not in docs/)

1. **Fixed column widths**: Status=14, Priority=12, Time Elapsed=16, Session
   ID=40 characters. These values encode design choices about readability vs
   density tradeoffs (e.g., why 40 chars for UUID when only 36 needed). The
   column order rationale is in `session-sorting.md` and memory, but the
   specific widths and their justification are not in any decision doc. (memory
   line ~25)

2. **History panel shows per-state duration, not "time ago"**: The decision to
   show how long a session spent in each state (e.g., "5m32s working →
   attention") rather than when each transition occurred (e.g., "3 minutes ago")
   is a UX design choice. Not captured in any decision doc. (memory line ~85)

3. **API usage widget width threshold (≥30 chars = long mode, <30 = compact)**:
   The widget switches between long format (`5h: 42% / 75% | ...`) and compact
   format (`[5h:8% 7d:77%]`) at 30 characters. This threshold value is a design
   decision. Not in any decision doc. (memory line ~147)

4. **Status symbols are ASCII characters**: The symbols `*`, `!`, `?`, `x`, `.`
   were chosen over Unicode symbols explicitly for terminal compatibility. This
   is a design decision with user-visible consequences. Not documented anywhere
   in `docs/decisions/`. (memory line ~184)

5. **Focused chip brackets use same style as chip content**: The `[` and `]`
   around a focused chip must match the chip's color and bold modifier. The root
   cause analysis (the `]` was previously in the next chip's separator span with
   wrong DarkGray style) and the fix (chip renders its own `]`) establish a
   rendering invariant worth preserving. Not in any decision doc. (memory line
   ~162) _Note_: borderline — this is a bug fix with a rendering contract, not a
   product design decision. Flagged because the invariant ("chip renders its own
   closing bracket") will matter for future chip rendering changes.

### Collisions (contradicts existing doc)

1. **Version display location**: `tui-visual` memory states "Version display
   moved from footer bottom-right to **header right-aligned** (acd-mq6y, done).
   Footer bottom-right now reserved for API usage (acd-0i4i, future work)."
   (memory line ~67)

   However, `docs/decisions/version-display.md` states the version is displayed
   in the **"bottom-right corner of the footer row, NOT in the header"** and
   explicitly says the header "stays as plain 'Agent Console Dashboard' (no
   version)."

   The decision doc appears to be stale — it documents the pre-acd-mq6y state
   that was subsequently reversed. The user's project MEMORY.md also confirms
   the header placement is the current implemented state. The decision doc needs
   to be updated to reflect the acd-mq6y change.

---

## Summary Table

```csv
Agent,Gaps,Collisions
build-tooling,2,1
cli-features,2 (+ 1 possible),0
daemon-core,1 (+ 1 borderline),0
docs-api,0,0
hook-ipc,3,0
test-writer,0,0
tui-rendering,4,0
tui-visual,4 (+ 1 borderline),1
```

## Recommended Actions

### High priority (affects external users or wire format)

- **`agent_type` serialization format** (hook-ipc gap 1): Document in
  `ipc-protocol.md`. Hook authors depending on `"claudecode"` string value need
  this guaranteed.
- **Version display collision** (tui-visual collision 1): Update
  `version-display.md` to reflect the acd-mq6y change (header, not footer).

### Medium priority (affects future maintainers)

- **Detail panel always visible** (tui-rendering gap 1): Add to a TUI layout
  decision doc.
- **History shows duration not "time ago"** (tui-visual gap 2): Add to a TUI UX
  decision doc.
- **Status symbols are ASCII** (tui-visual gap 4): Add to a TUI design doc.
- **Enter key fires hook** (tui-rendering gap 3): Clarify in a TUI interaction
  model doc.
- **MSRV collision** (build-tooling collision 1): Update
  `implementation-defaults.md` to replace "latest stable Rust only" with the
  pinning policy.

### Low priority (implementation contracts)

- Hook subprocess polling vs `wait_with_output()` (hook-ipc gap 2)
- Restart command semantics (cli-features gap 1)
- Uninstall preserves config (cli-features gap 2)
- Column widths (tui-visual gap 1)
- API usage widget width threshold (tui-visual gap 3)
- Broadcast trigger rule (daemon-core gap 1)
