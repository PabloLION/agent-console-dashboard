# Story: Implement Main Dashboard Layout

**Story ID:** S015
**Epic:** [E004 - TUI Dashboard](../epic/E004-tui-dashboard.md)
**Status:** Draft
**Priority:** P1
**Estimated Points:** 5

## Description

As a user,
I want to see all my agent sessions displayed in a clear dashboard layout,
So that I can quickly understand the status of each session at a glance.

## Context

The main dashboard layout is the primary view users will see when running the TUI. It needs to display all tracked sessions from the daemon with their status, working directory, and time-since-attention (if applicable). The layout must adapt to different terminal sizes and integrate with the daemon via SUBSCRIBE for real-time updates.

The design follows the widget-based philosophy where the dashboard orchestrates multiple widgets. This story focuses on the core layout structure; individual widgets are implemented in E005.

## Implementation Details

### Technical Approach

1. Create `ui.rs` for rendering logic
2. Implement `dashboard.rs` view component
3. Connect to daemon via IPC client using SUBSCRIBE command
4. Render session list with status indicators and colors
5. Display header with title and help hints
6. Show footer with keyboard shortcuts
7. Handle different terminal widths (responsive design)

### Files to Modify

- `src/tui/mod.rs` - Add ui module exports
- `src/tui/ui.rs` - Main rendering orchestration
- `src/tui/views/mod.rs` - Views module entry
- `src/tui/views/dashboard.rs` - Main dashboard view
- `src/tui/app.rs` - Add session state, daemon connection

### Dependencies

- [S014 - Ratatui Application Scaffold](./S014-ratatui-application-scaffold.md) - Requires TUI foundation
- [S012 - SUBSCRIBE Command](./S012-subscribe-command.md) - Real-time session updates
- [S005 - Session Data Model](./S005-session-data-model.md) - Session struct definition

## Acceptance Criteria

- [ ] Given sessions exist in daemon, when TUI starts, then all sessions are displayed in the list
- [ ] Given a session status changes, when daemon sends update, then display updates within 100ms
- [ ] Given terminal width < 40 cols, when rendering, then session names are abbreviated
- [ ] Given terminal width 40-80 cols, when rendering, then standard display is shown
- [ ] Given terminal width > 80 cols, when rendering, then additional columns (session ID) are shown
- [ ] Given any session status, when rendering, then correct color is applied (green=working, yellow=attention, blue=question, gray=closed)
- [ ] Given a session needs attention, when rendering, then elapsed time is displayed (e.g., "2m34s")

## Testing Requirements

- [ ] Unit test: Session list renders correct number of rows
- [ ] Unit test: Color mapping returns correct color for each status
- [ ] Unit test: Responsive breakpoints apply correct layout
- [ ] Integration test: Real-time updates from daemon appear in TUI
- [ ] Integration test: Multiple sessions display correctly

## Out of Scope

- Keyboard navigation between sessions (S016)
- Session detail/expanded view (S017)
- Widget customization and layout presets (E005)
- API usage display (E009)

## Notes

### Layout Structure

```text
┌─ Agent Console Dashboard ──────────────────────────────────┐
│                                                            │
│  Sessions:                                                 │
│  ● proj-a      Working      ~/projects/proj-a              │
│  ○ proj-b      Attention    ~/projects/proj-b      2m34s   │
│  ? proj-c      Question     ~/projects/proj-c              │
│  × old-proj    Closed       ~/old/project                  │
│                                                            │
│  [j/k] Navigate  [Enter] Details  [r] Resurrect  [q] Quit  │
└────────────────────────────────────────────────────────────┘
```

### Status Symbols and Colors

| Status | Symbol | Color |
|--------|--------|-------|
| Working | `●` | Green |
| Attention | `○` | Yellow |
| Question | `?` | Blue |
| Closed | `×` | Gray |
| Error | `!` | Red |

### Project Structure Addition

```text
src/
├── tui/
│   ├── mod.rs
│   ├── app.rs
│   ├── event.rs
│   ├── ui.rs            # NEW: Rendering orchestration
│   └── views/
│       ├── mod.rs       # NEW: Views module
│       └── dashboard.rs # NEW: Main dashboard view
```

### Responsive Design Breakpoints

| Width | Columns Shown |
|-------|--------------|
| < 40 | Symbol, Name (truncated) |
| 40-80 | Symbol, Name, Status, Working Dir |
| > 80 | Symbol, Name, Status, Working Dir, Time, Session ID |

### Ratatui Layout Example

```rust
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem};

fn render_dashboard(frame: &mut Frame, sessions: &[Session]) {
    let area = frame.area();

    // Create main layout with header, content, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Header
            Constraint::Min(3),     // Session list
            Constraint::Length(1),  // Footer
        ])
        .split(area);

    // Render session list
    let items: Vec<ListItem> = sessions
        .iter()
        .map(|s| format_session_line(s, area.width))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Sessions"));

    frame.render_widget(list, chunks[1]);
}
```

### IPC Integration

The dashboard connects to the daemon using SUBSCRIBE to receive real-time updates:

```rust
// On startup
client.send(Command::Subscribe)?;

// In event loop
match client.recv() {
    Ok(Message::SessionUpdate(session)) => {
        app.update_session(session);
        // Trigger re-render
    }
    // ...
}
```
