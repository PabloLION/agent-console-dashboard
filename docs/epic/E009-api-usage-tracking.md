# Epic: API Usage Tracking

**Epic ID:** E009 **Status:** Draft **Priority:** Medium **Estimated Effort:** S

## Summary

Display API usage metrics in the TUI dashboard by consuming the `claude-usage`
crate (E011). The TUI fetches account-level quota data (5h/7d utilization)
directly — no daemon-side tracking or custom IPC commands needed.

## Goals

- Display account-level 5h and 7d quota utilization in the dashboard
- Fetch data via `claude_usage::get_usage()` on a periodic interval
- Show rate limit reset times

## User Value

Users need visibility into their API quota to know when they're approaching
limits. By showing 5-hour and 7-day utilization percentages directly in the
dashboard, users can pace their usage and avoid hitting rate limits.

## Stories

| Story ID                                               | Title                        | Priority | Status |
| ------------------------------------------------------ | ---------------------------- | -------- | ------ |
| [S009.01](../stories/S009.01-api-usage-data-model.md)  | Integrate claude-usage crate | P1       | Draft  |
| [S009.02](../stories/S009.02-api-usage-command.md)     | ~~IPC command~~ (removed)    | —        | Cut    |
| [S009.03](../stories/S009.03-api-usage-tui-display.md) | Display usage in TUI         | P1       | Draft  |

## Dependencies

- [E004 - TUI Dashboard](./E004-tui-dashboard.md) - TUI must exist to display
  usage data
- [E011 - Claude Usage Crate](./E011-claude-usage-crate.md) - Provides
  `get_usage()` API for account-level quota

## Acceptance Criteria

- [ ] TUI calls `claude_usage::get_usage()` periodically (e.g., every 5 minutes)
- [ ] Displays 5h and 7d utilization percentages
- [ ] Shows time until rate limit reset
- [ ] Handles credential/network errors gracefully (shows "unavailable")
- [ ] Unit tests for display formatting per
      [testing strategy](../decisions/testing-strategy.md)

## Technical Notes

### Simplified Architecture

The TUI calls the `claude-usage` crate directly, bypassing the daemon entirely:

```rust
use claude_usage::get_usage;

// In TUI tick handler (every 5 min)
match get_usage() {
    Ok(data) => {
        // data.five_hour.utilization, data.seven_day.utilization
        update_usage_widget(data);
    }
    Err(_) => show_unavailable(),
}
```

### Display Format

```text
Quota: 5h 8% | 7d 77% | resets 2h 15m
```

Compact:

```text
[5h:8% 7d:77%]
```

### What Was Removed

Per-session token tracking (input/output tokens, cost estimates) was removed
from scope. Claude Code does not currently expose per-session token counts via
hooks. Account-level quota from E011 provides the most actionable information.

Per-session tracking may be revisited if Claude Code adds token reporting to
hook payloads.

## Out of Scope

- Per-session token tracking (no data source available)
- Daemon-side usage aggregation
- Cost estimates
- IPC commands for usage data
