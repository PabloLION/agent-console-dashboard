# Development Log

Authoritative record of project progress, decisions, and state.

## Project Overview

Agent Console Dashboard (`acd`) — a Ratatui-based TUI that monitors Claude Code
sessions via a Unix socket daemon. Hook scripts notify the daemon of session
state changes; the TUI subscribes for live updates.

### Workspace

| Crate                     | Purpose                           | Binary                     |
| ------------------------- | --------------------------------- | -------------------------- |
| `agent-console-dashboard` | Main app: daemon, TUI, CLI, hooks | `acd`                      |
| `claude-usage`            | Standalone Claude API usage SDK   | library (crates.io v0.2.3) |

## Story Implementation Status

47 stories implemented across 13 epics. All 6 dependency layers complete.

### Dependency Layers (all ✅)

| Layer | Stories | Key Deliverables                                                                     |
| ----- | ------- | ------------------------------------------------------------------------------------ |
| 0     | 9       | App scaffold, config schema, XDG, usage crate, health check, service files           |
| 1     | 9       | Dashboard layout, widget trait, hooks, config loading, session metadata, service CLI |
| 2     | 7       | Keyboard nav, status/dir/usage widgets, hook docs, default config                    |
| 3     | 3       | Detail view, layout presets, resurrect command                                       |
| 4     | 1       | Zellij layout files                                                                  |
| 5     | 1       | Terminal environment detection and executor                                          |
| 6     | 1       | Zellij resurrection workflow                                                         |

### Previously Completed (16)

S001.01–04, S002.01–04, S003.01–06, S011.01–06, S012.01

### Cut/Deferred

| Story   | Disposition      | Reason                                       |
| ------- | ---------------- | -------------------------------------------- |
| S008.03 | Moved to S010.03 | Consolidated                                 |
| S009.02 | Cut              | Daemon owns data (architectural decision D3) |
| S011.07 | Deferred         | Scheduled for v2+                            |

## Architecture

### Data Flow

```text
Claude Code hook fires
  → scripts/hooks/*.sh reads JSON stdin, extracts session_id
  → acd set <session_id> <status>
  → Unix socket → daemon SocketServer
  → SessionStore (thread-safe HashMap + broadcast channel)
  → TUI subscriber receives UPDATE messages
  → Ratatui renders live session status
```

### Key Architectural Decisions

- D1: Ratatui over tui-rs (active maintenance)
- D2: Unix domain sockets for IPC (zero config, sub-ms latency)
- D3: Daemon owns all session data (single source of truth)
- D4: Hook scripts use `jq` for JSON parsing (ubiquitous dependency)
- D5: Auto-start daemon from TUI via `connect_with_auto_start()`

## Commit History

### End-to-End Wiring

- `2e981f9` feat: wire end-to-end IPC between daemon, CLI, and TUI

### Layer 4–6 (Zellij + Terminal)

- `8e991be` feat: add Zellij resurrection workflow (S010.02)
- `4ef1591` feat: add terminal environment detection and executor (S010.03)
- `5506c6a` feat: add Zellij layout files and launcher script (S010.01)

### Layer 3 (Detail View + Presets)

- `f1e9b89` feat: implement RESURRECT IPC command and CLI (S008.02)
- `5568e5f` feat: implement layout presets (S005.05)
- `5741c6f` feat: implement session detail view (S004.04)

### Layer 0–2 (Foundation)

- `02c5b93` docs: add dependency graph, hook registration, and service setup
  guides
- `64ea15a` feat: implement TUI and widgets (S004.02-03, S005.01-04, S009.03)
- `74400ba` feat: implement config system (S007.01-04)
- `1b9a8e7` feat: implement Layer 0-1 stories (scaffold, config deps, daemon,
  hooks, service)

### Documentation Audit

- `1da92ef` docs: fix all 112 story alignment concerns from audit
- `d5760c4` docs: align 47 stories with updated epics and add concerns audit
- `9cfc2d2` docs: propagate architectural decisions into all 13 epics
- `766f3e4` docs: remove stale ApiUsage/api_usage references from epics and
  stories
- `3641a00` docs: rewrite S005.04 and fix stale token/cost references in epics
- `e139aa2` docs: fix epic path references and add clarity notes

## Current State (2026-02-01)

### What Works

- All 47 stories implemented with unit tests (473+ tests passing)
- Daemon starts socket server, accepts connections, manages sessions
- TUI connects to daemon, subscribes to live updates, renders dashboard
- `acd set` CLI command exists for hook scripts to call
- `acd status`, `acd dump`, `acd resurrect` CLI commands work
- Hook scripts parse Claude Code JSON stdin and forward to daemon
- Zellij layouts and launcher script ready
- Config system with XDG support, TOML schema, defaults
- Service install/uninstall for launchd (macOS) and systemd (Linux)

### What's Missing

- **Hook installation**: No automated way to register hooks in
  `~/.claude/settings.json`. Users must manually edit the file. Next task:
  `claude-hook-install` crate.
- **End-to-end validation**: Not tested with a real Claude Code instance yet.
- **Branch state**: All work is on `docs/create-missing-stories` (ephemeral, no
  upstream). Needs merge to `master`.

## Next Steps

### Immediate: `claude-hook-install` Crate

Standalone package (like `claude-usage`) to install/uninstall hooks into Claude
Code's settings.json.

Target location: `crates/claude-hook-install/`

Scope:

- Read/merge/write `~/.claude/settings.json` (global), `.claude/settings.json`
  (project), `.claude/settings.local.json` (local)
- Support all 12 hook events: SessionStart, UserPromptSubmit, PreToolUse,
  PermissionRequest, PostToolUse, PostToolUseFailure, Notification,
  SubagentStart, SubagentStop, Stop, PreCompact, SessionEnd
- Handler types: command, prompt, agent
- Handler fields: command, prompt, model, timeout, async, statusMessage, once
- Matcher support per event type
- CLI: `claude-hook-install install/uninstall/list/status`
- Library API for programmatic use from `acd hooks install`

Reference: `~/base/manifold/llm-agents/` may have related patterns (not yet
reviewed).

### After That

- Wire `acd hooks install/uninstall/status` subcommands using the new crate
- End-to-end testing with real Claude Code
- Merge branch to master
- Consider v0.1.0 release
