# Epic: Widget System

**Epic ID:** E005
**Status:** Draft
**Priority:** High
**Estimated Effort:** M

## Summary

Implement a modular widget-based architecture for the terminal UI where each line of output is a configurable widget. Users can select which widgets to display and in what order, enabling customizable dashboard layouts from minimal one-line views to comprehensive multi-line displays.

## Goals

- Create a widget trait/interface that defines the contract for all UI widgets
- Implement core widgets: session-status, working-dir, api-usage, state-history, clock, and spacer
- Build a layout system with predefined presets (one-line, two-line, detailed, history)
- Support custom user-defined layouts via configuration

## User Value

Users gain flexibility in how they monitor their agent sessions. Power users can create information-dense layouts, while those wanting minimal distraction can use compact one-line displays. The widget system also enables future extensibility - new data sources can be added as new widgets without disrupting existing functionality. Layouts can be switched instantly via keyboard shortcuts to adapt to different workflows.

## Stories

| Story ID | Title | Priority | Status |
|----------|-------|----------|--------|
| [S018](../stories/S018-widget-trait-interface.md) | Create widget trait/interface | P1 | Draft |
| [S019](../stories/S019-session-status-widget.md) | Implement session-status widget | P1 | Draft |
| [S020](../stories/S020-working-dir-widget.md) | Implement working-dir widget | P2 | Draft |
| [S021](../stories/S021-api-usage-widget.md) | Implement api-usage widget | P2 | Draft |
| [S022](../stories/S022-layout-presets.md) | Add layout presets (one-line, two-line, detailed) | P1 | Draft |

## Dependencies

- [E002 - Session Management](./E002-session-management.md) - Session data for session-status widget
- [E009 - API Usage Tracking](./E009-api-usage-tracking.md) - Usage data for api-usage widget

## Acceptance Criteria

- [ ] Widget trait defines render method that outputs a single line of styled text
- [ ] All core widgets implement the widget trait consistently
- [ ] Widgets handle terminal width constraints (truncation, abbreviation)
- [ ] Layout presets are configurable via TOML configuration
- [ ] Users can switch between layouts via keyboard shortcuts (1-4)
- [ ] Custom layouts can be defined in user configuration
- [ ] Widgets support both horizontal and vertical orientation modes

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

| Widget ID | Content | Min Width |
|-----------|---------|-----------|
| `working-dir` | Current working directory | 20 |
| `session-status` | All sessions with status and elapsed time | 30 |
| `session-detail` | Expanded view of selected session | 40 |
| `api-usage` | Token counts and cost estimate | 25 |
| `state-history` | Recent state transitions | 30 |
| `clock` | Current time | 8 |
| `spacer` | Empty line for visual separation | 0 |

### Layout Presets

| Layout | Widgets | Use Case |
|--------|---------|----------|
| `one-line` | `session-status` | Minimal, v1 compatible |
| `two-line` | `working-dir`, `session-status` | Standard |
| `detailed` | `working-dir`, `session-status`, `api-usage` | Full monitoring |
| `history` | `session-status`, `state-history` | Debug/analysis |

### Configuration Examples

```toml
# Simple widget list
[ui]
widgets = ["session-status", "api-usage"]

# Layout presets
[ui.layouts.one-line]
widgets = ["session-status"]

[ui.layouts.two-line]
widgets = ["working-dir", "session-status"]

[ui.layouts.detailed]
widgets = ["working-dir", "session-status", "api-usage", "state-history"]

# Custom layout
[ui.layouts.my-layout]
widgets = ["clock", "session-status", "spacer", "api-usage"]

# Orientation and display mode
[ui]
orientation = "vertical"  # or "horizontal"
display_mode = "full"     # or "compact"
```

### Display Modes

**Horizontal (default):** Sessions inline separated by `|`
```text
proj-a: - | proj-b: 2m34s | proj-c: ?
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
proj-a: - | proj-b: 2m34s | proj-c: ?
```

**API Usage Widget:**
```text
Tokens: 12.3k in / 8.1k out | $0.42 est
```

### Source Files

```text
src/
├── widgets/
│   ├── mod.rs           # Widget trait and registry
│   ├── session_status.rs
│   ├── working_dir.rs
│   ├── api_usage.rs
│   ├── state_history.rs
│   ├── clock.rs
│   └── spacer.rs
├── layout/
│   ├── mod.rs           # Layout manager
│   ├── presets.rs       # Built-in layouts
│   └── config.rs        # Layout configuration
```
