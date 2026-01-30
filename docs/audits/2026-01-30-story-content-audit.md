# Story Content Audit

**Date:** 2026-01-30 **Scope:** All 55 story files in docs/stories/ **Method:**
Automated review by 4 parallel agents checking against 9 quality criteria

## Audit Criteria

1. Description quality (user story format)
2. Context section (explains WHY)
3. Implementation details accuracy (file paths match codebase)
4. Acceptance criteria (testable, specific)
5. Dependencies (cross-references accurate)
6. Scope consistency with epic
7. Points estimation (reasonable relative sizing)
8. Code examples (correct crate/module names)
9. Out of scope (makes sense)

## Systemic Issues

### 1. File path prefix mismatch (30+ stories)

**Severity:** High **Affected:** E001-E005, E007-E010 stories

All stories reference `src/` paths (e.g., `src/daemon/server.rs`) but the actual
codebase uses `crates/agent-console-dashboard/src/` (workspace structure).
E011-E012 stories are correct because they were written after the workspace
restructure.

**Fix:** Global find-and-replace `src/` → `crates/agent-console-dashboard/src/`
in implementation details sections, with manual review for paths that should
stay as-is (e.g., `scripts/`, `docs/`).

### 2. E009 stories contradict revised epic (3 stories)

**Severity:** Critical **Affected:** S009.01, S009.02, S009.03

The E009 epic was revised to use `claude-usage` crate (E011) for account-level
quota display. All three stories still describe the original per-session token
tracking, IPC commands, and cost estimation approach. S009.02 was explicitly cut
from the epic but retains full Draft content.

**Fix:** Rewrite S009.01 to describe `claude-usage` integration, stub S009.02 as
cut, rewrite S009.03 to match the 5h/7d quota display format.

### 3. Non-existent module references in code examples (10+ stories)

**Severity:** Medium **Affected:** E001-E005 stories

Code examples reference modules like `crate::session::Session`,
`crate::api_usage::ApiUsage` that don't exist. Actual types live in
`daemon::store::` and the `claude-usage` crate.

### 4. Points inflation (8+ stories)

**Severity:** Low **Affected:** S001.01 (5→3), S002.04 (5→3), S004.01 (5→3),
S004.04 (5→3), S005.01 (5→2-3), S005.02 (5→3), S008.02 (5→3)

Several stories are over-estimated relative to their scope. Scaffold/boilerplate
stories rated at 5 should be 3.

## Per-Epic Findings

### E001 - Daemon Core Infrastructure (4 stories)

| Story   | Status | Issues                                                           |
| ------- | ------ | ---------------------------------------------------------------- |
| S001.01 | ⚠️     | File paths; 5→3 points; generic description                      |
| S001.02 | ⚠️     | File paths; "As a daemon" not valid actor                        |
| S001.03 | ⚠️     | File paths; "As a daemon"; scope overlap with S002.01 data model |
| S001.04 | ⚠️     | File paths only                                                  |

- **Data model duplication:** S001.03 and S002.01 both fully define
  Session/Status/StateTransition types
- **Inverted dependency:** S002.01 claims it depends on S001.03 (store) but data
  model should come before store
- **Circular dependency:** S002.04 needs subscriber infra from S003.04, but E003
  depends on E002

### E002 - Session Management (4 stories)

| Story   | Status | Issues                                                     |
| ------- | ------ | ---------------------------------------------------------- |
| S002.01 | ⚠️     | File paths; inverted dependency on S001.03; scope overlap  |
| S002.02 | ⚠️     | File paths; `session.rs` does not exist                    |
| S002.03 | ⚠️     | File paths; `config.rs` does not exist; code inconsistency |
| S002.04 | ⚠️     | File paths; 5→3 points; circular dependency with E003      |

### E003 - IPC Protocol & Client (6 stories)

| Story   | Status | Issues                                   |
| ------- | ------ | ---------------------------------------- |
| S003.01 | ⚠️     | File paths; `protocol.rs` does not exist |
| S003.02 | ⚠️     | File paths                               |
| S003.03 | ⚠️     | File paths only                          |
| S003.04 | ⚠️     | File paths only                          |
| S003.05 | ⚠️     | File paths; `commands.rs` does not exist |
| S003.06 | ⚠️     | File paths; `tests/` dir does not exist  |

### E004 - TUI Dashboard (4 stories)

| Story   | Status | Issues                                           |
| ------- | ------ | ------------------------------------------------ |
| S004.01 | ⚠️     | File paths; 5→3 points; outdated ratatui version |
| S004.02 | ⚠️     | File paths                                       |
| S004.03 | ✅     | Clean                                            |
| S004.04 | ⚠️     | File paths; 5→3 points                           |

### E005 - Widget System (5 stories)

| Story   | Status | Issues                                              |
| ------- | ------ | --------------------------------------------------- |
| S005.01 | ⚠️     | File paths; 5→2-3 points; wrong module refs in code |
| S005.02 | ⚠️     | File paths; 5→3 points; wrong module refs           |
| S005.03 | ⚠️     | File paths; `dirs` crate not in dependencies        |
| S005.04 | ⚠️     | File paths; `crate::api_usage` does not exist       |
| S005.05 | ⚠️     | File paths only; 5 points justified                 |

### E006 - Claude Code Integration (4 stories)

| Story   | Status | Issues                                                   |
| ------- | ------ | -------------------------------------------------------- |
| S006.01 | ⚠️     | Stale AskUserQuestion out-of-scope note contradicts epic |
| S006.02 | ✅     | Clean                                                    |
| S006.03 | ⚠️     | Stale AskQuestion note contradicts epic                  |
| S006.04 | ⚠️     | settings.json format incomplete vs epic                  |

### E007 - Configuration System (4 stories)

| Story   | Status | Issues                                              |
| ------- | ------ | --------------------------------------------------- |
| S007.01 | ⚠️     | File paths (expected for Draft)                     |
| S007.02 | ⚠️     | Missing hot-reload story leaves epic AC unfulfilled |
| S007.03 | ✅     | Clean                                               |
| S007.04 | ⚠️     | 2 points may be low given CLI scope                 |

### E008 - Session Resurrection (3 stories)

| Story   | Status | Issues                                                    |
| ------- | ------ | --------------------------------------------------------- |
| S008.01 | ⚠️     | Stale S008.03 reference (moved to E010)                   |
| S008.02 | ⚠️     | Multiple stale S008.03 refs; 5→3 points                   |
| S008.03 | ⚠️     | Moved story retains full content; should be stub redirect |

### E009 - API Usage Tracking (3 stories) -- CRITICAL

| Story   | Status | Issues                                              |
| ------- | ------ | --------------------------------------------------- |
| S009.01 | ❌     | Content contradicts revised epic direction entirely |
| S009.02 | ❌     | Cut per epic but file has full Draft content        |
| S009.03 | ❌     | Mismatched architecture, stale deps on cut story    |

### E010 - Zellij Integration (3 stories)

| Story   | Status | Issues                                                        |
| ------- | ------ | ------------------------------------------------------------- |
| S010.01 | ⚠️     | 3 points possibly low for scope                               |
| S010.02 | ⚠️     | File paths; title says "Document" but scope is implementation |
| S010.03 | ⚠️     | Overlap with S010.02 on Zellij integration module             |

### E011 - Claude Usage Crate (8 stories)

| Story   | Status | Issues                                           |
| ------- | ------ | ------------------------------------------------ |
| S011.01 | ✅     | Clean (Done)                                     |
| S011.02 | ✅     | Clean (Done)                                     |
| S011.03 | ✅     | Clean (Done)                                     |
| S011.04 | ✅     | Clean (Done)                                     |
| S011.05 | ✅     | Clean (Done)                                     |
| S011.06 | ✅     | Clean (Done)                                     |
| S011.07 | ⚠️     | `npm/` dir does not exist (Deferred, acceptable) |
| S011.08 | ⚠️     | Stale `docs/plans/6-open-questions.md` reference |

### E012 - Logging and Diagnostics (3 stories)

| Story   | Status | Issues                                                         |
| ------- | ------ | -------------------------------------------------------------- |
| S012.01 | ✅     | Clean (Done)                                                   |
| S012.02 | ⚠️     | Status mismatch: story says "Ready for Dev", epic says "Draft" |
| S012.03 | ⚠️     | Status mismatch; `--format text` in both impl and out-of-scope |

### E013 - Deployment Infrastructure (4 stories)

| Story   | Status | Issues                                                 |
| ------- | ------ | ------------------------------------------------------ |
| S013.01 | ⚠️     | `resources/` dir does not exist (acceptable for Draft) |
| S013.02 | ⚠️     | Same as S013.01                                        |
| S013.03 | ⚠️     | `acd service status` in both epic and out-of-scope     |
| S013.04 | ⚠️     | `docs/guides/` does not exist (acceptable for Draft)   |

## Summary Statistics

| Category                       | Count        |
| ------------------------------ | ------------ |
| Stories audited                | 55           |
| Clean (no issues)              | 11           |
| Minor issues (file paths only) | 20           |
| Moderate issues                | 21           |
| Critical issues                | 3 (all E009) |

## Recommended Fix Priority

1. **P0:** Rewrite E009 stories to match revised epic
2. **P1:** Fix `src/` → `crates/agent-console-dashboard/src/` path prefix
   globally
3. **P2:** Stub S008.03 as redirect to S010.03
4. **P2:** Fix stale cross-references (S008.01, S008.02 → S010.03)
5. **P3:** Update stale AskUserQuestion notes in S006.01, S006.03
6. **P3:** Resolve S012.02/S012.03 status mismatches
7. **P3:** Resolve S012.03 `--format text` contradiction
8. **P4:** Adjust point estimates where noted
9. **P4:** Fix S010.02 title/scope mismatch
10. **P4:** Clarify S010.02/S010.03 boundary
