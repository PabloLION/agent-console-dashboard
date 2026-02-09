# Epic: Widget System

**Epic ID:** E005 **Status:** Draft **Priority:** High **Estimated Effort:** M

## Summary

Implement a modular widget-based architecture for the terminal UI where each
line of output is a configurable widget. Users can select which widgets to
display and in what order, enabling customizable dashboard layouts from minimal
one-line views to comprehensive multi-line displays.

## Goals

- Create a widget trait/interface that defines the contract for all UI widgets
- Implement core widgets: session-status, working-dir, api-usage
- Build a layout system with two presets (default, compact)
- Defer custom user-defined layouts to v2+

## User Value

Users gain flexibility in how they monitor their agent sessions. Power users can
create information-dense layouts, while those wanting minimal distraction can
use compact one-line displays. The widget system also enables future
extensibility - new data sources can be added as new widgets without disrupting
existing functionality. Layouts can be switched instantly via keyboard shortcuts
to adapt to different workflows.

## Stories

| Story ID                                                | Title                                             | Priority | Status |
| ------------------------------------------------------- | ------------------------------------------------- | -------- | ------ |
| [S005.01](../stories/S005.01-widget-trait-interface.md) | Create widget trait/interface                     | P1       | Draft  |
| [S005.02](../stories/S005.02-session-status-widget.md)  | Implement session-status widget                   | P1       | Draft  |
| [S005.03](../stories/S005.03-working-dir-widget.md)     | Implement working-dir widget                      | P2       | Draft  |
| [S005.04](../stories/S005.04-api-usage-widget.md)       | Implement api-usage widget                        | P2       | Draft  |
| [S005.05](../stories/S005.05-layout-presets.md)         | Add layout presets (one-line, two-line, detailed) | P1       | Draft  |

## Dependencies

- [E002 - Session Management](./E002-session-management.md) - Session data for
  session-status widget
- [E009 - API Usage Tracking](./E009-api-usage-tracking.md) - Usage data for
  api-usage widget

## Acceptance Criteria

- [ ] Widget trait defines render method that outputs a single line of styled
      text
- [ ] All core widgets implement the widget trait consistently
- [ ] Widgets handle terminal width constraints (truncation, abbreviation)
- [ ] Layout presets are configurable via TOML configuration
- [ ] Users can switch between layouts via keyboard shortcuts (`1` default, `2`
      compact)
- [ ] Custom layouts deferred to v2+
- [ ] Widgets support both horizontal and vertical orientation modes
- [ ] Unit tests for widget rendering at various widths per
      [testing strategy](../decisions/testing-strategy.md)

## Technical Notes

### Widget Trait Design

```rust
pub trait Widget {
    /// Render the widget to a single line of styled text
    fn render(&self, width: u16, context: &WidgetContext) -> Line<'_>;

    /// Widget identifier for configuration
    fn id(&self) -> &'static str;

    /// Minimum width required for meaningful display
    fn min_width(&self) -> u16;
}
```

### Available Widgets

| Widget ID        | Content                                   | Min Width |
| ---------------- | ----------------------------------------- | --------- |
| `working-dir`    | Current working directory                 | 20        |
| `session-status` | All sessions with status and elapsed time | 30        |
| `session-detail` | Expanded view of selected session         | 40        |
| `api-usage`      | Account-level 5h/7d quota utilization     | 18        |

### Future Widgets (v2+)

The following widgets are deferred to v2+:

- `state-history` - Recent state transitions
- `clock` - Current time
- `spacer` - Empty line for visual separation

### Layout Presets

| Layout    | Widgets                                | Use Case |
| --------- | -------------------------------------- | -------- |
| `default` | `session-status:two-line`, `api-usage` | Standard |
| `compact` | `session-status:one-line`, `api-usage` | Minimal  |

Custom layout configuration and additional presets are deferred to v2+.

### Configuration Examples

```toml
[tui.layout]
preset = "default"  # default | compact

[tui.layout.presets.default]
widgets = ["session-status:two-line", "api-usage"]

[tui.layout.presets.compact]
widgets = ["session-status:one-line", "api-usage"]
```

### Display Modes

**Horizontal (default):** Sessions inline separated by `|`

```text
proj-a: ● | proj-b: 2m34s | proj-c: ?
```

**Vertical:** Each session on its own line

```text
proj-a    working     -
proj-b    attention   2m34s
proj-c    question    -
```

### Widget Mockups

**Working Directory Widget:**

```text
~/projects/my-app
```

**Session Status Widget:**

```text
proj-a: ● | proj-b: 2m34s | proj-c: ?
```

**API Usage Widget:**

```text
Quota: 5h 8% | 7d 77% | resets 2h 15m
```

### Source Files

```text
crates/agent-console-dashboard/
├── src/
│   ├── widgets/
│   │   ├── mod.rs           # Widget trait and registry
│   │   ├── session_status.rs
│   │   ├── working_dir.rs
│   │   └── api_usage.rs
│   └── layout/
│       ├── mod.rs           # Layout manager
│       ├── presets.rs       # Built-in layouts
│       └── config.rs        # Layout configuration
```

### Widget Data Sources

**Fully centralized architecture.** All widgets receive data exclusively via
`WidgetContext`. No widget makes external API calls or accesses the daemon
socket directly. See [widget data flow](../architecture/widget-data-flow.md).

- **session-status, working-dir**: Session data from daemon (received via TUI's
  SUBSCRIBE connection)
- **api-usage**: Usage data from daemon (daemon fetches from claude-usage crate
  every 3 minutes, broadcasts to subscribers)

Widgets are **stateless renderers**: `(WidgetContext, width) → Line`. They must
not cache data, maintain timers, or depend on other widgets.
