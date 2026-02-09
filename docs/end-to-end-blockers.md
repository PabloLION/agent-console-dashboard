# End-to-End Blockers

What prevents ACD from working as a complete system today. Each blocker includes
the problem, evidence, and a concrete fix.

## Current Hook Coverage

ACD registers 6 hooks via `acd install` (writes to `~/.claude/settings.json`):

| Hook                                  | Status mapped | When it fires              |
| ------------------------------------- | ------------- | -------------------------- |
| `SessionStart`                        | `attention`   | Session begins or resumes  |
| `UserPromptSubmit`                    | `working`     | User sends a prompt        |
| `Stop`                                | `attention`   | Claude finishes responding |
| `SessionEnd`                          | `closed`      | Session ends               |
| `Notification` (`elicitation_dialog`) | `question`    | AskUserQuestion dialog     |
| `Notification` (`permission_prompt`)  | `attention`   | Permission prompt          |

Source: <https://code.claude.com/docs/en/hooks>

## Blockers

### B1: Plugin not installed in Claude Code

**Status:** Partially resolved (2026-02-08)

**Problem:** `.claude-plugin/` is a gitignored build artifact at the workspace
root. Claude Code does not auto-discover it.

**Resolution (personal use):** `acd install` writes hooks directly to
`~/.claude/settings.json`. Works across all projects immediately. Implemented in
acd-qe8.

**Still open (distribution):** The plugin marketplace path is unclear.
Publishing the plugin for `claude plugin install agent-console-dashboard`
requires a marketplace entry. The `.claude-plugin/` build artifact is gitignored
and won't ship via git clone. A distribution strategy decision is needed (see
B4).

Source:
<https://code.claude.com/docs/en/plugins-reference#plugin-installation-scopes>

### B2: `acd` binary not in PATH

**Status:** Resolved (2026-02-08)

`acd install` checks if `acd` is in `$PATH` and warns if not.
`cargo install --path crates/agent-console-dashboard` puts the binary at
`~/.cargo/bin/acd`. Implemented in acd-qe8.

### B3: Missing hook events

**Status:** Resolved (2026-02-08)

SessionEnd, Notification (elicitation_dialog), and Notification
(permission_prompt) hooks added to both `build.rs` (plugin path) and `main.rs`
(settings.json path). The `claude-hook` subcommand accepts `closed` and
`question` as valid status values. Implemented in acd-2n6.

All 6 hook events now registered (see hook coverage table above).

### B4: Plugin distribution strategy

**Status:** Open

Two hook installation paths exist:

| Path                         | Method                              | Audience     |
| ---------------------------- | ----------------------------------- | ------------ |
| `acd install`                | Writes to `~/.claude/settings.json` | Personal use |
| `.claude-plugin/plugin.json` | Build artifact from `build.rs`      | Distribution |

**Decision needed:** Keep both paths, or remove the plugin approach?

- The plugin path requires publishing to a marketplace or manual
  `claude --plugin-dir` usage
- `.claude-plugin/` is gitignored — won't ship via git clone
- `acd install` works for all use cases today
- Plugin approach only matters if distributing to others

### B5: No root README

**Status:** Resolved (2026-02-08)

README.md created with overview, installation, setup, usage, architecture, and
development guide. PR #3 on branch `chore/readme-and-beads-sync`. Implemented in
acd-35c.

### B6: No `acd install` command

**Status:** Resolved (2026-02-08)

`acd install` writes 6 hooks to `~/.claude/settings.json`, verifies `acd` is in
PATH, and prints next steps. `acd uninstall` removes hooks cleanly. Both are
idempotent. Implemented in acd-qe8 and acd-71n.

E2E smoke test passed: `cargo install` → `acd install` → hooks fire → daemon
lazy-starts → sessions tracked → `acd uninstall`.

### B7: Socket path not configurable via hooks

**Problem:** The default socket `/tmp/agent-console-dashboard.sock` is hardcoded
in the CLI default. Hook commands don't pass `--socket`, so they always use the
default. This works for single-user setups but won't scale to multi-user or
custom configurations.

**Fix:** Low priority. Default works for personal use.

## End-to-End User Journey (Target)

```text
1. cargo install --path crates/agent-console-dashboard
   (puts `acd` in ~/.cargo/bin/)

2. acd install
   (writes hooks to ~/.claude/settings.json, verifies PATH)

3. Restart Claude Code (or start new session)

4. Claude Code fires hooks automatically:
   SessionStart    → acd claude-hook attention  → daemon lazy-starts
   UserPromptSubmit → acd claude-hook working
   Stop            → acd claude-hook attention
   Notification    → acd claude-hook question/attention
   SessionEnd      → acd claude-hook closed

5. acd tui
   (shows live dashboard of all sessions)

6. acd uninstall
   (removes hooks from ~/.claude/settings.json)
```

## What Already Works

- Daemon lifecycle (spawn, socket, sessions, shutdown)
- IPC protocol (SET, STATUS, DUMP, RESURRECT, LIST, SUB)
- TUI rendering (sessions, status, timestamps, usage, column headers, mouse)
- Lazy-start (daemon auto-spawns from hooks or TUI)
- Config system (TOML from XDG paths)
- All CLI subcommands parse and execute correctly
- Inactive session detection (1 hour threshold)
- Build system generates plugin.json with version sync
- `acd install` / `acd uninstall` for hook management
- All 6 hook events registered (SessionStart, UserPromptSubmit, Stop,
  SessionEnd, Notification/elicitation_dialog, Notification/permission_prompt)
- README.md with installation and usage guide
- E2E smoke test verified (2026-02-08)

## Non-Goals

- **System service (launchd/systemd):** Deliberately avoided. Lazy-start from
  hooks replaces the need for a persistent daemon. The daemon spawns on first
  hook event and stays running. No service management required.
