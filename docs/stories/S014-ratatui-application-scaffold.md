# Story: Create Ratatui Application Scaffold

**Story ID:** S014 **Epic:**
[E004 - TUI Dashboard](../epic/E004-tui-dashboard.md) **Status:** Draft
**Priority:** P1 **Estimated Points:** 5

## Description

As a developer, I want to create a Ratatui application scaffold with proper
terminal initialization, So that I have a foundation for building the
interactive TUI dashboard.

## Context

The TUI dashboard is the primary user interface for the Agent Console Dashboard.
It needs to be built on Ratatui (a Rust library for building terminal user
interfaces) with Crossterm as the terminal backend. This scaffold establishes
the basic application structure including terminal setup/teardown, the main
event loop, and proper panic handling to ensure terminal state is always
restored.

The scaffold follows the standard Ratatui application pattern with:

- Raw mode terminal configuration
- Alternate screen buffer
- Event-driven architecture with Tokio async runtime
- Clean shutdown handling

## Implementation Details

### Technical Approach

1. Add Ratatui and Crossterm dependencies to Cargo.toml
2. Create `tui/mod.rs` module structure
3. Implement terminal initialization (raw mode, alternate screen)
4. Create application state struct to hold TUI state
5. Implement main event loop with async event handling
6. Add panic hook to restore terminal state on crashes
7. Implement clean shutdown with terminal restoration

### Files to Modify

- `Cargo.toml` - Add ratatui, crossterm dependencies
- `src/main.rs` - Add `tui` subcommand
- `src/tui/mod.rs` - TUI module entry point
- `src/tui/app.rs` - Application state and main loop
- `src/tui/event.rs` - Event handling infrastructure

### Dependencies

- [S001 - Create Daemon Process](./S001-create-daemon-process.md) - Shares CLI
  entry point
- [S009 - IPC Message Protocol](./S009-ipc-message-protocol.md) - Will need IPC
  client for daemon connection

## Acceptance Criteria

- [ ] Given the TUI is started, when terminal is accessed, then raw mode is
      enabled and alternate screen is active
- [ ] Given the TUI is running, when `q` is pressed, then the application exits
      cleanly and terminal is restored
- [ ] Given the TUI is running, when a panic occurs, then the panic hook
      restores terminal state before crashing
- [ ] Given the TUI is running, when terminal resize event occurs, then the
      application receives the resize event
- [ ] Given the TUI is started, then startup time is under 100ms
- [ ] Given the TUI exits, then no terminal corruption occurs (cursor visible,
      echo enabled)

## Testing Requirements

- [ ] Unit test: Application state initializes correctly
- [ ] Unit test: Event channel creation succeeds
- [ ] Integration test: TUI starts and exits cleanly in CI environment
- [ ] Integration test: Panic hook restores terminal (spawn subprocess, force
      panic)

## Out of Scope

- Main dashboard layout rendering (S015)
- Keyboard navigation (S016)
- Session detail view (S017)
- Widget system integration (E005)
- Actual daemon connection (deferred to S015)

## Notes

### Project Structure

```text
src/
├── main.rs              # CLI entry, tui subcommand added
├── tui/
│   ├── mod.rs           # TUI module entry, re-exports
│   ├── app.rs           # Application state, event loop
│   └── event.rs         # Event handling (keyboard, resize, tick)
```

### Key Dependencies

| Crate     | Version | Purpose          |
| --------- | ------- | ---------------- |
| ratatui   | 0.26+   | TUI framework    |
| crossterm | 0.27+   | Terminal backend |
| tokio     | 1.x     | Async runtime    |

### CLI Interface

```bash
# Start TUI dashboard (foreground)
agent-console tui

# Start with specific layout
agent-console tui --layout detailed

# Start with custom socket path
agent-console tui --socket /tmp/agent-console.sock
```

### Terminal Initialization Pattern

```rust
use crossterm::{
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};
use ratatui::prelude::*;

pub fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

pub fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
```

### Event Loop Architecture

The event loop should:

1. Poll for terminal events (keyboard, resize) via crossterm
2. Handle IPC updates from daemon (via SUBSCRIBE channel)
3. Trigger periodic UI refreshes if needed
4. Process application state changes and re-render
