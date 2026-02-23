# Resurrect → Reopen Migration

Created: 20260215T030000Z Issue: acd-bwa (parent), sub-issues acd-3qx, acd-puk,
acd-jau, acd-0hx

## Problem

`acd resurrect <id>` was a CLI command that looked up a closed session,
validated the working directory, and returned `claude --resume <id>` as text.
The user had to copy-paste it. The TUI had a TODO (Action::Resurrect) that
didn't work yet.

Problems with this design:

- Copy-pasting a command is bad UX
- The `integrations/zellij` module tried to be multiplexer-aware (detect
  Zellij/tmux, run command in new pane) — coupling ACD to specific tools
- "Resurrect" is the wrong word — we reopen a closed session

## Solution

Replace the entire resurrect system with configurable hooks.

### Naming

"Resurrect" → "reopen" throughout the codebase. The daemon IPC command is
`REOPEN`.

### Daemon (acd-3qx)

- `IpcCommand::Resurrect` → `IpcCommand::Reopen`
- `SessionStore::reopen_session()` moves a session from closed queue → active
  sessions map
- Status always set to `Attention` on reopen (semantically accurate — user needs
  to interact with the reopened session)
- Returns `SessionSnapshot` on success

### Config (acd-puk)

Two hook arrays replace `double_click_hook`:

- `tui.activate_hooks` — fires on double-click/Enter of a non-closed session
  (renamed from `double_click_hook`, extended to array format)
- `tui.reopen_hooks` — fires on double-click/Enter/r of a closed session (new)

Both receive `SessionSnapshot` as JSON on stdin and environment variables
`$ACD_SESSION_ID`, `$ACD_WORKING_DIR`, `$ACD_STATUS`.

The gesture is the same (double-click or Enter). The TUI decides which hook to
fire based on session state.

### CLI (acd-jau)

The `acd resurrect` command was removed entirely. Reopen is TUI-only — no CLI
replacement needed. Users configure hooks to handle reopen in their preferred
way (e.g., `claude --resume {session_id}` in a new terminal pane).

### Cleanup (acd-0hx)

The `integrations/zellij` module was deleted. ACD no longer needs to know about
terminal multiplexers — the hook command handles that.

The `terminal/` module (environment detection + executor) will be deleted next
(acd-3le) since its only consumer was `integrations/zellij`.

## Design Decisions

### Why hooks over built-in multiplexer support

Hooks are generic. The user configures a shell command that does whatever their
setup needs. ACD doesn't need to add support for every multiplexer. A Zellij
user writes `zellij run -- claude --resume $ACD_SESSION_ID`, a tmux user writes
`tmux new-window claude --resume $ACD_SESSION_ID`.

### Why Attention status on reopen

The reopened session will get a proper status from the next Claude Code hook
event (e.g., SessionStart). Setting Attention is semantically accurate because
the user needs to interact with it. Traversing history to find the last
non-closed status would add complexity with no benefit.

### Why reopen is TUI-only (no CLI command)

The reopen workflow is interactive — user sees a closed session, double-clicks
it, hook fires. A CLI command for this adds complexity without clear use cases.
If CLI reopen is needed later, it can be added as `acd session reopen <id>`.

## Files Changed

- `src/ipc.rs` — IPC command rename
- `src/daemon/handlers/mod.rs` — handler rename + reopen implementation
- `src/daemon/store/lifecycle.rs` — `reopen_session()` method
- `src/daemon/store/tests/lifecycle_reopen.rs` — new test file (9 tests)
- `src/config/schema.rs` — `activate_hooks` + `reopen_hooks` fields (array of
  HookConfig)
- `src/config/default.rs` — default template updated
- `src/main.rs` — resurrect command removed, config wiring updated
- `src/commands/ipc.rs` — `run_resurrect_command()` removed
- `src/integrations/` — deleted
- `src/lib.rs` — `pub mod integrations;` removed
