# Story: Implement Session Status Widget

**Story ID:** S019
**Epic:** [E005 - Widget System](../epic/E005-widget-system.md)
**Status:** Draft
**Priority:** P1
**Estimated Points:** 5

## Description

As a user,
I want to see all my agent sessions with their status at a glance,
So that I can quickly identify which sessions need attention.

## Context

The session-status widget is the core widget of the Agent Console Dashboard. It displays all tracked sessions in a compact format, showing session names alongside their current status and elapsed time if awaiting attention. This is the most important widget and must be highly efficient as it updates frequently.

The widget supports both horizontal (inline) and vertical (one per line) display modes. It also uses color coding to indicate session state: green for working, yellow for attention, blue for question, and gray for closed.

## Implementation Details

### Technical Approach

1. Create `src/widgets/session_status.rs`
2. Implement Widget trait for SessionStatusWidget
3. Support horizontal format: `proj-a: - | proj-b: 2m34s | proj-c: ?`
4. Support vertical format with status column alignment
5. Apply color styles based on session status
6. Handle width constraints with name truncation
7. Show elapsed time for sessions awaiting attention

### Files to Modify

- `src/widgets/session_status.rs` - SessionStatusWidget implementation
- `src/widgets/mod.rs` - Export and register session-status widget

### Dependencies

- [S018 - Widget Trait/Interface](./S018-widget-trait-interface.md) - Widget trait to implement
- [S005 - Session Data Model](./S005-session-data-model.md) - Session struct definition
- [S006 - Session Status Transitions](./S006-session-status-transitions.md) - Status types and colors

## Acceptance Criteria

- [ ] Given multiple sessions exist, when widget renders in horizontal mode, then all sessions are shown separated by `|`
- [ ] Given a session is Working, when displayed, then symbol is `-` in green color
- [ ] Given a session is Attention, when displayed, then elapsed time is shown (e.g., `2m34s`) in yellow
- [ ] Given a session is Question (AskUserQuestion), when displayed, then symbol is `?` in blue
- [ ] Given a session is Closed, when displayed, then symbol is `×` in gray
- [ ] Given terminal width is limited, when session names don't fit, then names are truncated with ellipsis
- [ ] Given a session is selected, when widget renders, then selected session is visually highlighted
- [ ] Given vertical orientation is configured, when widget renders, then each session is on its own line

## Testing Requirements

- [ ] Unit test: Horizontal format renders sessions correctly with separators
- [ ] Unit test: Vertical format renders one session per line with alignment
- [ ] Unit test: Status colors are applied correctly for each state
- [ ] Unit test: Elapsed time formatting is human-readable (2m34s, 1h5m, etc.)
- [ ] Unit test: Name truncation works correctly at various widths
- [ ] Unit test: Empty session list renders gracefully

## Out of Scope

- Session detail view (S017 in E004)
- Session selection highlighting (handled by layout/TUI layer)
- Resurrection actions (E008)
- Session filtering or sorting

## Notes

### Display Formats

**Horizontal (default):**
```text
proj-a: - | proj-b: 2m34s | proj-c: ?
```

**Vertical (compact mode):**
```text
proj-a    -           -
proj-b    attention   2m
proj-c    ?           -
```

**Vertical (full mode):**
```text
proj-a    working     -
proj-b    attention   waited 2m
proj-c    question    -
```

### Status Symbols and Colors

| Status | Symbol | Color | Example |
|--------|--------|-------|---------|
| Working | `-` | Green | `proj-a: -` |
| Attention | `Xm` | Yellow | `proj-b: 2m34s` |
| Question | `?` | Blue | `proj-c: ?` |
| Closed | `×` | Gray | `old-proj: ×` |

### Implementation

```rust
use ratatui::prelude::*;
use crate::widgets::{Widget, WidgetContext};
use crate::session::{Session, SessionStatus};

pub struct SessionStatusWidget {
    orientation: Orientation,
    display_mode: DisplayMode,
}

#[derive(Clone, Copy)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy)]
pub enum DisplayMode {
    Compact,
    Full,
}

impl SessionStatusWidget {
    pub fn new() -> Self {
        Self {
            orientation: Orientation::Horizontal,
            display_mode: DisplayMode::Compact,
        }
    }

    fn render_horizontal(&self, width: u16, context: &WidgetContext) -> Line<'_> {
        let mut spans = Vec::new();

        for (i, session) in context.sessions.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" | "));
            }

            spans.push(Span::raw(format!("{}: ", session.name)));
            spans.push(self.status_span(session));
        }

        Line::from(spans)
    }

    fn status_span(&self, session: &Session) -> Span<'_> {
        match session.status {
            SessionStatus::Working => {
                Span::styled("-", Style::default().fg(Color::Green))
            }
            SessionStatus::Attention { since } => {
                let elapsed = format_elapsed(since);
                Span::styled(elapsed, Style::default().fg(Color::Yellow))
            }
            SessionStatus::Question => {
                Span::styled("?", Style::default().fg(Color::Blue))
            }
            SessionStatus::Closed => {
                Span::styled("×", Style::default().fg(Color::Gray))
            }
        }
    }
}

impl Widget for SessionStatusWidget {
    fn render(&self, width: u16, context: &WidgetContext) -> Line<'_> {
        match self.orientation {
            Orientation::Horizontal => self.render_horizontal(width, context),
            Orientation::Vertical => self.render_vertical(width, context),
        }
    }

    fn id(&self) -> &'static str {
        "session-status"
    }

    fn min_width(&self) -> u16 {
        30
    }
}
```

### Elapsed Time Formatting

```rust
fn format_elapsed(since: DateTime<Local>) -> String {
    let duration = Local::now() - since;
    let secs = duration.num_seconds();

    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m{}s", secs / 60, secs % 60)
    } else {
        format!("{}h{}m", secs / 3600, (secs % 3600) / 60)
    }
}
```

### Name Truncation

When horizontal space is limited, truncate session names:
- Keep at least 3 characters of name
- Add `…` suffix for truncated names
- Priority: never truncate status indicators, only names
