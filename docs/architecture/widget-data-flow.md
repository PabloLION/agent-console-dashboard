# Widget Data Flow Architecture

**Created:** 2026-01-31 **Status:** Active **Related Decisions:** D3, D4 in
[2026-01-31-discussion-decisions.md](./2026-01-31-discussion-decisions.md)

## Overview

The widget system follows a fully centralized data flow architecture where the
daemon is the single source of truth for ALL data. Widgets are stateless
renderers that only read from `WidgetContext` — no widget ever makes external
API calls or maintains its own state.

**Principle:** Daemon owns all data fetching, transformation, and broadcasting.
TUI is a pure presentation layer.

## Data Flow Diagram

```text
External Sources
┌──────────────────┐        ┌──────────────────┐
│ claude-usage API │        │ Hook Events      │
│ (Anthropic)      │        │ (JSON stdin)     │
└────────┬─────────┘        └────────┬─────────┘
         │ every 3 min               │ on event
         ▼                           ▼
┌─────────────────────────────────────────────┐
│           DAEMON (Single Source)            │
│  ┌─────────────────────────────────────┐   │
│  │ Sessions (HashMap<SessionId, ...>)  │   │
│  └─────────────────────────────────────┘   │
│  ┌─────────────────────────────────────┐   │
│  │ Usage Data (from claude-usage)      │   │
│  │  - five_hour / seven_day %          │   │
│  │  - reset_time                       │   │
│  │  - status: Ok | Unavailable         │   │
│  └─────────────────────────────────────┘   │
└──────────────────┬──────────────────────────┘
                   │ broadcast (on change)
                   ▼
┌─────────────────────────────────────────────┐
│       N × TUI Dashboards (subscribers)     │
│  ┌─────────────────────────────────────┐   │
│  │ Populate WidgetContext from daemon   │   │
│  │  - sessions: Vec<Session>           │   │
│  │  - usage: UsageData                 │   │
│  │  - now: Instant (local clock)       │   │
│  │  - selected_index: Option<usize>    │   │
│  └──────────────────┬──────────────────┘   │
│                     ▼                       │
│  ┌─────────────────────────────────────┐   │
│  │ Widgets (stateless renderers)       │   │
│  │  session-status.render(ctx) → Line  │   │
│  │  api-usage.render(ctx) → Line       │   │
│  │  working-dir.render(ctx) → Line     │   │
│  │  clock.render(ctx) → Line           │   │
│  └─────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
```

## Data Sources

### Session Data

**Source:** Hook events via JSON stdin **Flow:** Hook → daemon socket → update
HashMap → broadcast to TUIs **Update trigger:** Event-driven (immediate)

### Usage Data

**Source:** `claude-usage` crate (`claude_usage::get_usage()`) **Flow:** Daemon
timer → fetch → store → broadcast to TUIs **Update trigger:** Time-based (every
3 minutes)

**Fetch interval rationale:**

- 5-hour window = 300 minutes
- 1% of 300 minutes = 3 minutes
- Fetching every 3 minutes aligns with 1% accuracy granularity
- Displayed percentage can be at most 1% stale

**Conditional fetching:** Daemon only fetches when ≥1 TUI is subscribed. No
audience = no API calls. (Temporary decision; see beads issue `acd-j4u`.)

### Local Data

**Source:** TUI internal state and system clock **Examples:** `now: Instant`,
`selected_index: Option<usize>`, terminal width **Flow:** TUI populates directly
in WidgetContext, no daemon involvement

## Rationale for Centralized Architecture

If each TUI called `claude_usage::get_usage()` directly:

- **Wasteful:** N TUIs × periodic API calls = N × fetch overhead
- **Rate limiting risk:** Multiple concurrent calls to Anthropic API
- **Inconsistent data:** Each TUI might see different snapshots

Daemon fetches once, broadcasts to all subscribers:

- **Efficient:** 1 API call serves N TUIs
- **Rate limit safe:** Single fetch point, easy to control frequency
- **Consistent:** All TUIs see identical data at same time

## Widget Contract

Widgets are pure functions: `(WidgetContext, width) → Line`

```rust
pub trait Widget: Send + Sync {
    fn render(&self, width: u16, context: &WidgetContext) -> Line<'_>;
    fn id(&self) -> &'static str;
    fn min_width(&self) -> u16;
}

pub struct WidgetContext {
    pub sessions: Vec<Session>,
    pub usage: UsageData,
    pub now: Instant,
    pub selected_index: Option<usize>,
}
```

### Widget Constraints

Widgets MUST:

- Be stateless (no internal mutable state)
- Only read from `WidgetContext` parameter
- Never make external API calls
- Never directly access daemon socket
- Handle missing/unavailable data gracefully (show placeholder)

Widgets MUST NOT:

- Cache data between renders
- Call `claude_usage::get_usage()` directly
- Maintain timers or async tasks
- Depend on other widgets

## Error Handling

### Usage Fetch Failures

1. Daemon logs error
2. Daemon broadcasts `UsageData { status: Unavailable }` to TUIs
3. `api-usage` widget displays "unavailable" placeholder
4. Daemon retries on next 3-minute interval
5. No cascading failures — session tracking unaffected

### Daemon Crash Recovery

1. TUI detects socket disconnect → shows "disconnected" indicator
2. Next hook event → daemon auto-starts
3. Sessions re-register via hook events
4. Usage fetch resumes on next interval
5. TUI reconnects, normal display resumes

## Configuration

```toml
[daemon]
usage_fetch_interval = "3m"  # default: 3 minutes
```

Valid values: `"1m"`, `"3m"`, `"5m"`, `"10m"`

**Warning:** Interval < 1 minute risks rate limiting from Anthropic API.

## Related Documentation

- [E005 - Widget System](../epic/E005-widget-system.md)
- [E009 - API Usage Tracking](../epic/E009-api-usage-tracking.md)
- [Discussion Decisions D3, D4](./2026-01-31-discussion-decisions.md)
- [Decision Amendments 3, 4](./2026-01-31-decision-amendments.md)
