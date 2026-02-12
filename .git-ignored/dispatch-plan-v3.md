# Dispatch Plan v3 — Named Agents with Persistent Memory

## Orchestration Rules

**Main thread (you) does NOT write code.** Main thread:

- Creates/manages worktrees
- Dispatches agents with clear task descriptions
- Reviews agent output for spec drift
- Merges branches back to main
- Handles 1-2 line fixes only
- Manages context, beads sync, and git operations

**Agents do ALL code work.** Each agent:

- Works in its own git worktree (`/tmp/acd-wt-<agent-name>`)
- Makes atomic commits per issue
- Runs `cargo test` + `cargo clippy` before each commit
- Checks its persistent memory before starting
- Updates its persistent memory after completing
- Does NOT push — main thread handles merges

## Agent Definitions

All agents defined in `.claude/agents/` with `memory: project`.

## 12-Agent Dispatch

| # | Agent Name | Issues (sequential) | Files Touched | Notes |
|---|-----------|---------------------|---------------|-------|
| 1 | tui-visual | acd-87o → acd-0hd | dashboard.rs (styles, layout) | Highlight fix + narrow layout |
| 2 | docs-design | acd-g40 | docs/design/ui.md | Selection interaction model |
| 3 | docs-api | acd-9n5 → acd-dmy | main.rs + lib.rs (doc comments only) | Daemon start docs + SessionSnapshot docs |
| 4 | daemon-validation | acd-8vx → acd-rhr | daemon/store, handlers | Validate cwd path + UUID v4 validation |
| 5 | hook-research | acd-0ab → acd-h3h | research + docs output | Resume hooks + hot-reload research |
| 6 | hook-ipc | acd-79b → acd-7jh | app.rs, lib.rs, config | Double-click hook from config + SessionSnapshot stdin JSON |
| 7 | debug-infra | acd-l1b | tui/ event handling | Interaction logging infrastructure |
| 8 | test-writer | acd-0ci → acd-i71 | tests/, test_utils.rs | E2E basename test + remove unused helper |
| 9 | build-tooling | acd-ir7 → acd-6gn | .git/hooks/, claude-usage crate | Pre-commit auto-fix + snake_case warnings |
| 10 | tui-rendering | acd-6o2 → acd-7xz → acd-pdx | tui/app.rs tick, dashboard.rs rendering | 1fps passive + ellipsis + dynamic padding |
| 11 | cli-features | acd-50g → acd-lj1 → acd-qga → acd-8vg → acd-rzl | main.rs CLI sections | Install output + uninstall + config feedback + config edit + autocompletion |
| 12 | mock-testing | acd-0f0 | daemon/store, test infra | Mock sessions with custom timestamps |

Total: 26 issues across 12 agents.

## Overlap Analysis

- Agents 1 & 10: both touch dashboard.rs — different areas (styles vs rendering)
- Agents 3 & 6: both touch lib.rs — #3 doc comments only, #6 code changes
- Agent 11 touches main.rs broadly — isolated in worktree, no conflict

All overlaps handled by git worktrees.

## Research Agents (5, 12)

Agents 5 and 12 produce research reports and documentation, not just code.
Their output goes to `.git-ignored/` or `docs/` as appropriate.
Main thread reviews research output and discusses with user.

## Merge Order

1. Non-overlapping agents first (2, 3, 4, 5, 7, 8, 9, 12)
2. Then dashboard.rs agents (1, then 10)
3. Then main.rs agents (11, then 6 if needed)
4. Run full test suite after each merge
5. Final `cargo test && cargo clippy` on main

## Rendering Clarification (acd-6o2)

Two rendering modes for agent 10:

- **Passive updates** (daemon data, elapsed time): 1 frame per second
- **User input** (click, scroll, keyboard): real-time, immediate response

This is selective throttling, not a global cap.
