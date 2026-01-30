# Epic: TUI Dashboard

**Epic ID:** E004 **Status:** Draft **Priority:** High **Estimated Effort:** L

## Summary

Build the terminal user interface (TUI) dashboard application using Ratatui that
provides a real-time visual display of all agent sessions. The dashboard
connects to the daemon via IPC, renders session status, supports keyboard
navigation, and allows users to interact with sessions through an intuitive
terminal interface.

## Goals

- Create a responsive Ratatui application scaffold with proper terminal
  initialization
- Implement the main dashboard layout with session list and status display
- Add keyboard navigation for session selection and quick actions
- Build session detail view for expanded information and actions

## User Value

Users get a powerful, always-visible terminal dashboard to monitor all their
Claude Code agent sessions at a glance. The keyboard-driven interface allows
quick navigation without leaving the terminal, and the real-time updates ensure
users never miss when an agent needs attention. The minimal TUI design fits
perfectly in a Zellij/tmux pane without consuming excessive screen space.

## Stories

| Story ID                                                       | Title                                       | Priority | Status |
| -------------------------------------------------------------- | ------------------------------------------- | -------- | ------ |
| [S004.01](../stories/S004.01-ratatui-application-scaffold.md)  | Create Ratatui application scaffold         | P1       | Draft  |
| [S004.02](../stories/S004.02-main-dashboard-layout.md)         | Implement main dashboard layout             | P1       | Draft  |
| [S004.03](../stories/S004.03-keyboard-navigation.md)           | Add keyboard navigation                     | P1       | Draft  |
| [S004.04](../stories/S004.04-session-selection-detail-view.md) | Implement session selection and detail view | P2       | Draft  |

## Dependencies

- [E001 - Daemon Core Infrastructure](./E001-daemon-core-infrastructure.md) -
  Requires running daemon
- [E003 - IPC Protocol & Client](./E003-ipc-protocol-and-client.md) - Requires
  SUBSCRIBE command for real-time updates
- [E005 - Widget System](./E005-widget-system.md) - Widget components used by
  dashboard

## Acceptance Criteria

- [ ] TUI application starts and renders properly in terminal
- [ ] Dashboard displays all sessions from daemon with correct status indicators
- [ ] Real-time updates appear without manual refresh via SUBSCRIBE
- [ ] Keyboard navigation (j/k) moves between sessions
- [ ] Enter key expands session detail view
- [ ] Quick actions (r for resurrect, d for remove) work from main view
- [ ] Application handles terminal resize events gracefully
- [ ] Clean exit with 'q' key restores terminal state
- [ ] Unit tests for event handling and app state per
      [testing strategy](../decisions/testing-strategy.md)

## Technical Notes

### Technology Stack

| Component        | Technology         |
| ---------------- | ------------------ |
| TUI Framework    | Ratatui            |
| Terminal Backend | Crossterm          |
| Async Runtime    | Tokio              |
| IPC Client       | Unix domain socket |

### Application Architecture

```text
crates/agent-console-dashboard/
├── src/
│   └── tui/
│       ├── mod.rs           # TUI module entry
│       ├── app.rs           # Application state and event loop
│       ├── ui.rs            # UI rendering logic
│       ├── event.rs         # Event handling (keyboard, resize)
│       └── views/
│           ├── mod.rs
│           ├── dashboard.rs # Main dashboard view
│           └── detail.rs    # Session detail view
```

### Keyboard Shortcuts

| Key     | Action                    |
| ------- | ------------------------- |
| `j/k`   | Navigate sessions up/down |
| `Enter` | Expand session detail     |
| `r`     | Resurrect closed session  |
| `d`     | Remove session from list  |
| `1-4`   | Switch layout preset      |
| `q`     | Quit application          |
| `?`     | Show help                 |

### Color Scheme

| Status    | Color  |
| --------- | ------ |
| Working   | Green  |
| Attention | Yellow |
| Question  | Blue   |
| Closed    | Gray   |
| Error     | Red    |

### Responsive Design

The TUI adapts to terminal width:

| Width    | Behavior                                   |
| -------- | ------------------------------------------ |
| <40 cols | Abbreviate session names, hide details     |
| 40-80    | Standard display                           |
| >80      | Show additional columns (session ID, etc.) |

### Mock-up: Full TUI

```text
┌─ Agent Console Dashboard ──────────────────────────────────┐
│                                                            │
│  Sessions:                                                 │
│  ● proj-a      Working      ~/projects/proj-a              │
│  ○ proj-b      Attention    ~/projects/proj-b      2m34s   │
│  ? proj-c      Question     ~/projects/proj-c              │
│  × old-proj    Closed       ~/old/project                  │
│                                                            │
│  Quota: 5h 8% | 7d 77% | resets 2h 15m                    │
│                                                            │
│  [j/k] Navigate  [Enter] Details  [r] Resurrect  [q] Quit  │
└────────────────────────────────────────────────────────────┘
```

### Integration with Zellij

The TUI is designed to run in a dedicated Zellij pane, receiving resize events
automatically via crossterm's terminal event detection.
