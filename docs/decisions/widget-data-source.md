# Decision: Widget Data Source

**Decided:** 2026-01-31 **Status:** Implemented

## Context

The original design had most widgets reading from `WidgetContext` (populated by
the daemon) but the api-usage widget calling `claude_usage::get_usage()`
directly from the TUI process. With multiple TUI instances, this meant N API
calls per fetch interval.

## Decision

The daemon is the single source of truth for ALL data. TUI dashboards only talk
to the daemon. Widgets only read from `WidgetContext`.

```text
claude-usage crate -> daemon (fetches every 3 min) -> broadcast to TUIs
hooks (JSON stdin) -> daemon (session state)        -> broadcast to TUIs
TUI receives all data -> populates WidgetContext    -> passes to widgets
```

### Fetch Interval

The daemon fetches API usage every **3 minutes** (configurable via
`[daemon] usage_fetch_interval = "3m"`).

The 3-minute interval was derived from the 5-hour quota window: 5h = 300 min,
and 1% of 300 minutes = 3 minutes. This means the displayed percentage is at
most 1% stale.

### Conditional Fetching

The daemon only fetches usage when at least one TUI is subscribed. No audience
means no API calls.

## Rationale

- Centralized fetch prevents N TUIs from making N API calls
- Reduces risk of rate limiting from Anthropic's API
- Consistent architecture with no special cases per widget

## Amendments

- Amendment 3 (2026-01-31): fully centralized widget data through daemon
- Amendment 4 (2026-01-31): changed fetch interval from 5 minutes to 3 minutes

See [D3, D4](../archive/planning/discussion-decisions.md) and
[Amendments 3, 4](../archive/planning/decision-amendments.md).

## Implementation

Tracked in E005 (widget system), E009 (API usage tracking), E004 (TUI
dashboard).

[D3, D4](../archive/planning/discussion-decisions.md) |
[Amendments 3, 4](../archive/planning/decision-amendments.md)
