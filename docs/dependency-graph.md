# Story Dependency Graph

Generated 2026-01-31. Stories marked ✅ are implemented (code exists).

## Workflow Note

**Compact context after completing each layer** to keep agent context
manageable.

## Execution Layers

Each layer can be executed in parallel. A layer can only start after all its
dependencies in previous layers are complete.

### Layer 0 — No blockers (9 stories) ✅ DONE

| Story   | Title                        | Status | Location                                   |
| ------- | ---------------------------- | ------ | ------------------------------------------ |
| S004.01 | Ratatui application scaffold | ✅     | `src/tui/app.rs`                           |
| S006.01 | Stop hook script             | ✅     | `scripts/hooks/stop.sh`                    |
| S007.01 | TOML configuration schema    | ✅     | `src/config/schema.rs`                     |
| S007.03 | XDG path support             | ✅     | `src/config/xdg.rs`                        |
| S009.01 | Integrate claude-usage crate | ✅     | `crates/claude-usage/`                     |
| S011.08 | Update E009 use claude-usage | ✅     | `src/daemon/usage.rs`                      |
| S012.02 | Health check command         | ✅     | `src/main.rs` (status cmd)                 |
| S013.01 | macOS launchd plist          | ✅     | `resources/com.agent-console.daemon.plist` |
| S013.02 | Linux systemd unit file      | ✅     | `resources/acd.service`                    |

### Layer 1 — Depends on Layer 0 (9 stories) ✅ DONE

| Story   | Title                     | Status | Location                              |
| ------- | ------------------------- | ------ | ------------------------------------- |
| S004.02 | Main dashboard layout     | ✅     | `src/tui/ui.rs`, `views/dashboard.rs` |
| S005.01 | Widget trait interface    | ✅     | `src/widgets/mod.rs`                  |
| S006.02 | User prompt submit hook   | ✅     | `scripts/hooks/user-prompt-submit.sh` |
| S006.03 | Notification hook script  | ✅     | `scripts/hooks/notification.sh`       |
| S007.02 | Configuration loading     | ✅     | `src/config/loader.rs`                |
| S008.01 | Closed session metadata   | ✅     | `src/daemon/session.rs`               |
| S012.03 | Diagnostic dump command   | ✅     | `src/main.rs` (dump cmd)              |
| S013.03 | Install/uninstall CLI     | ✅     | `src/service.rs`                      |
| S013.04 | Manual service setup docs | ⚪     | Docs-only by design                   |

### Layer 2 — Depends on Layer 1 (7 stories) ✅ DONE

| Story   | Title                      | Status | Location                                     |
| ------- | -------------------------- | ------ | -------------------------------------------- |
| S004.03 | Keyboard navigation        | ✅     | `src/tui/event.rs` (Action enum + routing)   |
| S005.02 | Session status widget      | ✅     | `src/widgets/session_status.rs`              |
| S005.03 | Working directory widget   | ✅     | `src/widgets/working_dir.rs`                 |
| S005.04 | API usage widget           | ✅     | `src/widgets/api_usage.rs`                   |
| S006.04 | Hook registration docs     | ✅     | `docs/integration/claude-code-hooks.md`      |
| S007.04 | Default configuration file | ✅     | `src/config/default.rs`                      |
| S009.03 | API usage TUI display      | ✅     | `src/widgets/api_usage.rs` (same as S005.04) |

### Layer 3 — Depends on Layer 2 (3 stories) ✅ DONE

| Story   | Title                         | Status | Location                          |
| ------- | ----------------------------- | ------ | --------------------------------- |
| S004.04 | Session selection detail view | ✅     | `src/tui/views/detail.rs`         |
| S005.05 | Layout presets                | ✅     | `src/layout/presets.rs`           |
| S008.02 | Resurrect command             | ✅     | `src/main.rs`, `daemon/server.rs` |

### Layer 4 — Depends on Layer 3 (1 story) ⏳ NEXT

| Story   | Title                   | Status | Blocking deps        |
| ------- | ----------------------- | ------ | -------------------- |
| S010.01 | Zellij layout dashboard | ⏳     | S004.01✅, S005.05✅ |

### Layer 5 — Depends on Layer 4 (1 story)

| Story   | Title                     | Status | Blocking deps      |
| ------- | ------------------------- | ------ | ------------------ |
| S010.03 | Claude resume in terminal | ⏳     | S008.02✅, S010.01 |

### Layer 6 — Depends on Layer 5 (1 story)

| Story   | Title                        | Status | Blocking deps |
| ------- | ---------------------------- | ------ | ------------- |
| S010.02 | Zellij resurrection workflow | ⏳     | S010.03       |

## Already Done (16 stories)

S001.01–04, S002.01–04, S003.01–06, S011.01–06, S012.01

## Cut/Deferred/Moved (3 stories)

- S008.03 — Moved to S010.03
- S009.02 — Cut (daemon owns data per D3)
- S011.07 — Deferred to v2+
