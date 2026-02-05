# Decision Amendments

**Date:** 2026-01-31 **Amends:** `docs/plans/7-decisions.md`

This document tracks changes to original decisions. The original decision doc
remains the source of truth for context and rationale. This file records what
changed and why.

## Amendment 1: Q25 — Auto-Stop Idle Threshold

**Original:** `AUTO_STOP_IDLE_THRESHOLD_SECS = 1800` (30 minutes) **Amended
to:** `AUTO_STOP_IDLE_THRESHOLD_SECS = 3600` (60 minutes) **Configurable:** Yes,
via `[daemon] idle_timeout = "60m"`

**Reason:** 60 minutes is more forgiving for intermittent usage. Combined with
auto-start, the debounce effect eliminates concern about rapid socket
create/delete cycles.

**Affects:**

- E001 (daemon core) — update auto-stop constants
- S001.04 (auto-start) — no change needed
- Code: `crates/agent-console-dashboard/src/daemon/` — update constant

## Amendment 2: Concurrency Model (implicit in Q1/E001)

**Original:** Implied `tokio::spawn` per connection + `Arc<RwLock<HashMap>>`
**Amended to:** Single-threaded actor model with mpsc queue + plain `HashMap`

**Reason:** Eliminates all race conditions and RwLock complexity. Sequential
message processing is sufficient for our scale (< 50 connections).

**Affects:**

- E001 (daemon core) — update technical approach
- E003 (IPC protocol) — update connection handling description
- Code: `crates/agent-console-dashboard/src/daemon/server.rs` — if exists

## Amendment 3: Widget Data Source (implicit in E005/E009)

**Original:** Most widgets via WidgetContext, api-usage widget calls
`claude_usage::get_usage()` directly from TUI **Amended to:** Fully centralized.
Daemon fetches usage every 3 minutes, broadcasts to TUIs. All widgets read from
WidgetContext only.

**Reason:** With N TUIs, decentralized = N API calls. Daemon fetches once,
broadcasts to all.

**Affects:**

- E005 (widget system) — update data flow description
- E009 (API usage tracking) — TUI no longer calls get_usage() directly
- E004 (TUI dashboard) — TUI receives usage data from daemon subscription

## Amendment 4: Usage Fetch Interval (new, related to E009)

**Original:** 5 minutes (mentioned in E009 code snippet) **Amended to:** 3
minutes

**Reason:** 5h window = 300 min. 1% = 3 min. Aligns fetch interval with 1%
accuracy granularity.

**Affects:**

- E009 (API usage tracking) — update interval references
- E001 (daemon core) — daemon now has usage fetch responsibility
