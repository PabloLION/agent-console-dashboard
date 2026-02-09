# Project Status

Current state of ACD as a production-ready system. Combines findings from the
end-to-end blockers audit, open questions resolution, and the 2026-02-08
production readiness analysis.

Last updated: 2026-02-08

## Recently Resolved

Items closed during the 2026-02-08 session:

| Item                       | What was done                                                                                        | Issue            |
| -------------------------- | ---------------------------------------------------------------------------------------------------- | ---------------- |
| Hook event coverage        | Added SessionEnd, Notification (elicitation_dialog, permission_prompt) to both build.rs and main.rs  | acd-2n6          |
| Install/uninstall commands | `acd install` writes 6 hooks to settings.json, `acd uninstall` removes them. PATH check, idempotent. | acd-qe8, acd-71n |
| Root README                | Overview, installation, setup, usage, architecture, development guide                                | acd-35c (PR #3)  |
| E2E smoke test             | Manual: cargo install → acd install → hooks fire → daemon → sessions tracked → uninstall             | —                |
| Branch protection          | Main branch protected, PRs required, CI checks enforced                                              | acd-36h          |
| AskUserQuestion behavior   | Confirmed elicitation_dialog notification type works correctly                                       | acd-g5h          |
| TUI error display          | `<error>` in red for missing cwd                                                                     | acd-lht          |
| TUI column alignment       | Flexbox-style: name expands, last 3 columns right-aligned                                            | acd-r57          |
| TUI column headers         | Fixed header row with Cyan bold styling                                                              | acd-8uw          |
| TUI mouse interaction      | Click, double-click, scroll wheel                                                                    | acd-3cv          |

## Currently Open

Real problems that need resolution. Each has a beads issue for tracking.

### Production Gaps

| Issue   | Problem                                                                                                          | Priority |
| ------- | ---------------------------------------------------------------------------------------------------------------- | -------- |
| acd-p6f | Config documentation and `acd config show` command. No user-facing docs on what config options exist.            | P2       |
| acd-4g0 | E2E integration test with real Claude Code. Manual smoke test passed but procedure not documented or repeatable. | P2       |
| acd-qk2 | Plugin distribution strategy (B1+B4). Two hook paths exist with no clear strategy for distribution to others.    | P3       |

### From End-to-End Blockers Doc

| Blocker | Status              | Detail                                                                                  |
| ------- | ------------------- | --------------------------------------------------------------------------------------- |
| B1      | Partially resolved  | `acd install` works for personal use. Plugin marketplace path unclear for distribution. |
| B4      | Open                | Decision needed: keep plugin.json for distribution AND settings.json for personal use?  |
| B7      | Open (low priority) | Socket path not configurable via hooks. Default works for single-user.                  |

### TUI Polish (Epic acd-cx9)

| Issue   | Story                        | Status                                                     |
| ------- | ---------------------------- | ---------------------------------------------------------- |
| acd-hex | Compact 3-line layout        | Open — `--layout compact` flag, auto-detect small terminal |
| acd-bwa | Wire TUI resurrect to daemon | Open — design TBD, needs hooks in config                   |

### Other Open Work

| Issue   | Description                                                                      | Priority |
| ------- | -------------------------------------------------------------------------------- | -------- |
| acd-j4u | Usage fetch timer: config `usage_fetch_interval` defined but not wired to daemon | P3       |
| acd-lj1 | Uninstall command enhancements                                                   | P3       |
| acd-uws | `--claude-path` flag for custom Claude Code binary location                      | P4       |
| acd-tvr | Hook JSON schema versioning                                                      | P4       |
| acd-51l | RwLock vs Actor model evaluation (deferred, design decision)                     | P4       |

## Deferred (v1+/v2+)

From open questions resolution tracking. These are intentionally deferred, not
forgotten:

| Topic                               | Decision                       | Source |
| ----------------------------------- | ------------------------------ | ------ |
| Zellij native plugin (WASM)         | Evaluate after v1              | Q8     |
| Tmux native plugin                  | On request only                | Q9     |
| Man pages                           | `clap_mangen` in v1+           | Q46    |
| Windows support                     | Named Pipes in v2+             | Q23    |
| Sound/notification on status change | v1+                            | Q105   |
| Dynamic session reorder             | v1+, stable order in v0        | Q91    |
| Theme/color customization           | v1+                            | Q75    |
| Label toggle (usage widget)         | v1+                            | Q88    |
| Auto-restart daemon on crash        | v2+, basic recovery only in v0 | Q24    |
| Feature flags                       | None for v0                    | Q49    |

## Non-Goals

Decisions explicitly made to NOT pursue:

| Item                             | Rationale                                                                       |
| -------------------------------- | ------------------------------------------------------------------------------- |
| System service (launchd/systemd) | Lazy-start from hooks replaces persistent daemon. Removed in acd-5dh by design. |
| Multi-user support               | Single-user tool, socket permissions 0600 (Q33)                                 |
| Remote access                    | Local-only by design                                                            |

## End-to-End User Journey (Current)

The working flow as of 2026-02-08:

```text
1. cargo install --path crates/agent-console-dashboard
   (puts `acd` in ~/.cargo/bin/)

2. acd install
   (writes 6 hooks to ~/.claude/settings.json, verifies PATH)

3. Restart Claude Code (or start new session)

4. Claude Code fires hooks automatically:
   SessionStart         → acd claude-hook attention  → daemon lazy-starts
   UserPromptSubmit     → acd claude-hook working
   Stop                 → acd claude-hook attention
   Notification (elicit)→ acd claude-hook question
   Notification (perm)  → acd claude-hook attention
   SessionEnd           → acd claude-hook closed

5. acd tui
   (shows live dashboard with columns, headers, mouse support)

6. acd uninstall
   (removes hooks from ~/.claude/settings.json)
```

## CLI Command Inventory

| Command                    | Purpose                                 | Status            |
| -------------------------- | --------------------------------------- | ----------------- |
| `acd tui`                  | Launch TUI dashboard                    | Working           |
| `acd daemon`               | Start daemon (foreground or daemonized) | Working           |
| `acd status`               | Check daemon health                     | Working           |
| `acd dump`                 | Export all sessions as JSON             | Working           |
| `acd set <id> <status>`    | Manually set session status             | Working           |
| `acd resurrect <id>`       | Get command to resume closed session    | Working           |
| `acd claude-hook <status>` | Hook handler (reads JSON from stdin)    | Working           |
| `acd config init`          | Create default config file              | Working           |
| `acd config path`          | Show config file path                   | Working           |
| `acd config validate`      | Validate config file                    | Working           |
| `acd config show`          | Display effective config values         | Missing (acd-p6f) |
| `acd install`              | Write hooks to settings.json            | Working           |
| `acd uninstall`            | Remove hooks from settings.json         | Working           |
