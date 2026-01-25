# Epic: API Usage Tracking

**Epic ID:** E009 **Status:** Draft **Priority:** Medium **Estimated Effort:** M

## Summary

Track and display API consumption metrics to provide users with visibility into
their Claude Code usage costs and limits. This epic covers the data model for
API usage, daemon commands for reporting usage, and TUI widgets for displaying
this information in real-time.

## Goals

- Define a data model for tracking API usage per session and cumulatively
- Implement IPC commands to report and query API usage metrics
- Display current session token usage in the dashboard
- Show cumulative usage across all sessions
- Surface rate limit status when available

## User Value

Users need visibility into their API consumption to manage costs effectively and
avoid unexpected billing. By showing real-time token usage for the current
session and cumulative usage across sessions, users can make informed decisions
about when to start new sessions, how to optimize their prompts, and whether
they are approaching rate limits. This transparency builds trust and helps users
budget their AI assistant usage appropriately.

## Stories

| Story ID                                         | Title                       | Priority | Status |
| ------------------------------------------------ | --------------------------- | -------- | ------ |
| [S034](../stories/S034-api-usage-data-model.md)  | Define API usage data model | P1       | Draft  |
| [S035](../stories/S035-api-usage-command.md)     | Implement API_USAGE command | P1       | Draft  |
| [S036](../stories/S036-api-usage-tui-display.md) | Display usage in TUI        | P2       | Draft  |

## Dependencies

- [E001 - Daemon Core Infrastructure](./E001-daemon-core-infrastructure.md) -
  Daemon must be running to store and aggregate usage data
- [E002 - Session Management](./E002-session-management.md) - Usage must be
  associated with specific sessions
- [E003 - IPC Protocol & Client](./E003-ipc-protocol-and-client.md) - API_USAGE
  command requires IPC protocol support
- [E005 - Widget System](./E005-widget-system.md) - TUI display uses the widget
  system for api-usage widget
- [E006 - Claude Code Integration](./E006-claude-code-integration.md) - Hooks
  may provide usage data from Claude Code
- [E011 - Claude Usage Crate](./E011-claude-usage-crate.md) - Provides
  account-level quota utilization data (5h/7d) via the `claude-usage` crate

## Acceptance Criteria

- [ ] API usage data model captures tokens (input/output), cost estimates, and
      timestamps
- [ ] Per-session usage is tracked and queryable via IPC command
- [ ] Cumulative usage across all sessions is calculated and available
- [ ] Rate limit status is displayed when information is available
- [ ] TUI displays usage metrics in a clear, readable format
- [ ] Usage data persists during daemon runtime (not across reboots per
      non-goals)

## Technical Notes

### Relationship with E011 (Claude Usage Crate)

This epic handles **per-session token tracking** (how many tokens each session
consumed), while E011 provides **account-level quota data** (5-hour and 7-day
utilization percentages from Anthropic's API).

| Scope   | Epic | Data Source   | Example                               |
| ------- | ---- | ------------- | ------------------------------------- |
| Session | E009 | Hooks/logs    | "This session used 12,000 tokens"     |
| Account | E011 | Anthropic API | "You've used 77% of your 7-day quota" |

The daemon periodically fetches account-level quota data using the
`claude-usage` crate:

```rust
use claude_usage::get_usage;

fn refresh_quota(&self) -> Result<(), Error> {
    let quota = get_usage()?;
    // quota.five_hour.utilization, quota.seven_day.utilization, etc.
    self.broadcast_quota_update(quota);
    Ok(())
}
```

### API Usage Data Model

Track usage at the session level with aggregation support:

| Field                | Type                | Description                      |
| -------------------- | ------------------- | -------------------------------- |
| session_id           | String              | Claude Code session identifier   |
| input_tokens         | u64                 | Total input tokens consumed      |
| output_tokens        | u64                 | Total output tokens generated    |
| total_tokens         | u64                 | Sum of input and output tokens   |
| estimated_cost       | f64                 | Estimated cost in USD (optional) |
| rate_limit_remaining | `Option<u32>`       | Remaining API calls if available |
| rate_limit_reset     | `Option<Timestamp>` | When rate limit resets           |
| updated_at           | Timestamp           | Last update time                 |

### IPC Commands

New commands to support API usage tracking:

```text
API_USAGE <session-id>
  Get API usage for a specific session
  Returns: JSON object with usage metrics

API_USAGE_ALL
  Get aggregated API usage across all sessions
  Returns: JSON object with cumulative metrics

SET_USAGE <session-id> <usage-json>
  Update usage metrics for a session (called by hooks)
  Returns: OK | ERROR <reason>
```

### Data Sources

API usage data may come from:

1. **Claude Code hooks** - If hooks provide usage information in their payload
2. **External log parsing** - Parsing Claude Code output logs (if available)
3. **Manual reporting** - User/script-provided usage data

### Display Format

The api-usage widget should show:

```text
Tokens: 1.2k / 5.3k (session/total)
Cost: ~$0.02
Rate: 98/100 remaining
```

Compact format for one-line layout:

```text
[1.2k/5.3k tok | $0.02 | 98 left]
```

### Limitations

- Token counts depend on Claude Code exposing this information
- Cost estimates are approximate based on published pricing
- Rate limit information may not be available from all sources
- Usage data is ephemeral (resets on daemon restart per project non-goals)

### Testing Strategy

- Unit tests for usage data model serialization/deserialization
- Unit tests for aggregation calculations
- Integration tests for API_USAGE commands
- Widget rendering tests with various usage scenarios
- Mock data for development and testing when Claude Code data is unavailable
