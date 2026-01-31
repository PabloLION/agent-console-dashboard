//! Application state and main event loop for the TUI.
//!
//! Manages terminal setup/teardown, panic hooks, and the core render loop.

use crate::tui::event::{handle_key_event, Action, Event, EventHandler};
use crate::tui::ui::render_dashboard;
use crate::Session;
use crossterm::{
    event::EventStream,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::{CrosstermBackend, Terminal};
use std::io::{self, stdout};
use std::path::PathBuf;
use std::time::Duration;

/// Core application state for the TUI.
#[derive(Debug)]
pub struct App {
    /// Whether the application should exit.
    pub should_quit: bool,
    /// Socket path for daemon IPC.
    pub socket_path: PathBuf,
    /// Count of ticks processed (useful for testing/diagnostics).
    pub tick_count: u64,
    /// Active and closed sessions displayed in the dashboard.
    pub sessions: Vec<Session>,
    /// Currently selected session index in the list.
    pub selected_index: Option<usize>,
}

impl App {
    /// Creates a new App with the given socket path.
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            should_quit: false,
            socket_path,
            tick_count: 0,
            sessions: Vec::new(),
            selected_index: None,
        }
    }

    /// Initializes `selected_index` to `Some(0)` if sessions exist, otherwise `None`.
    pub fn init_selection(&mut self) {
        self.selected_index = if self.sessions.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    /// Moves the selection down by one, clamped to the last session.
    pub fn select_next(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        let last = self.sessions.len().saturating_sub(1);
        self.selected_index = Some(self.selected_index.map_or(0, |i| (i + 1).min(last)));
    }

    /// Moves the selection up by one, clamped to index 0.
    pub fn select_previous(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        self.selected_index = Some(self.selected_index.map_or(0, |i| i.saturating_sub(1)));
    }

    /// Returns a reference to the currently selected session, if any.
    pub fn selected_session(&self) -> Option<&Session> {
        self.selected_index.and_then(|i| self.sessions.get(i))
    }

    /// Runs the TUI application: sets up terminal, enters event loop, restores on exit.
    pub async fn run(&mut self) -> io::Result<()> {
        // Install panic hook that restores terminal before printing panic info
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let _ = restore_terminal();
            original_hook(panic_info);
        }));

        setup_terminal()?;

        let result = self.event_loop().await;

        restore_terminal()?;
        result
    }

    /// Main event loop: renders UI and processes events.
    async fn event_loop(&mut self) -> io::Result<()> {
        let backend = CrosstermBackend::new(stdout());
        let mut terminal =
            Terminal::new(backend).expect("failed to create ratatui terminal instance");
        let event_handler = EventHandler::new(Duration::from_millis(250));
        let mut reader = EventStream::new();

        loop {
            // Render
            terminal.draw(|frame| {
                render_dashboard(frame, self);
            })?;

            // Handle events
            let event = event_handler.next(&mut reader).await?;
            match event {
                Event::Key(key) => {
                    match handle_key_event(self, key) {
                        Action::Quit => {
                            self.should_quit = true;
                            return Ok(());
                        }
                        Action::OpenDetail(idx) => {
                            tracing::debug!("open detail view for session index {idx}");
                            // TODO(S004.04): implement detail view overlay
                        }
                        Action::Resurrect(id) => {
                            tracing::debug!("resurrect session {id}");
                            // TODO(S008.02): send RESURRECT IPC command
                        }
                        Action::Remove(id) => {
                            tracing::debug!("remove session {id}");
                            // TODO: show confirmation, then send REMOVE IPC command
                        }
                        Action::SwitchLayout(preset) => {
                            tracing::debug!("switch to layout preset {preset}");
                            // TODO(S005.05): apply layout preset
                        }
                        Action::Back => {
                            tracing::debug!("back / close overlay");
                            // TODO(S004.04): close detail view if open
                        }
                        Action::None => {}
                    }
                }
                Event::Tick => {
                    self.tick_count += 1;
                }
                Event::Resize(_, _) => {
                    // Terminal auto-handles resize on next draw
                }
            }
        }
    }
}

/// Enables raw mode and switches to the alternate screen.
fn setup_terminal() -> io::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    Ok(())
}

/// Restores the terminal to its original state.
fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AgentType;

    fn make_app_with_sessions(count: usize) -> App {
        let mut app = App::new(PathBuf::from("/tmp/test.sock"));
        for i in 0..count {
            app.sessions.push(Session::new(
                format!("session-{}", i),
                AgentType::ClaudeCode,
                PathBuf::from(format!("/home/user/project-{}", i)),
            ));
        }
        app.init_selection();
        app
    }

    #[test]
    fn test_app_new() {
        let app = App::new(PathBuf::from("/tmp/test.sock"));
        assert!(!app.should_quit);
        assert_eq!(app.socket_path, PathBuf::from("/tmp/test.sock"));
        assert_eq!(app.tick_count, 0);
        assert!(app.sessions.is_empty());
        assert!(app.selected_index.is_none());
    }

    #[test]
    fn test_app_default_state() {
        let app = App::new(PathBuf::from("/tmp/agent-console.sock"));
        assert!(!app.should_quit);
        assert_eq!(app.tick_count, 0);
        assert!(app.sessions.is_empty());
        assert!(app.selected_index.is_none());
    }

    #[test]
    fn test_app_tick_increment() {
        let mut app = App::new(PathBuf::from("/tmp/test.sock"));
        assert_eq!(app.tick_count, 0);
        app.tick_count += 1;
        assert_eq!(app.tick_count, 1);
        app.tick_count += 1;
        assert_eq!(app.tick_count, 2);
    }

    #[test]
    fn test_app_should_quit_toggle() {
        let mut app = App::new(PathBuf::from("/tmp/test.sock"));
        assert!(!app.should_quit);
        app.should_quit = true;
        assert!(app.should_quit);
    }

    #[test]
    fn test_app_socket_path() {
        let app = App::new(PathBuf::from("/custom/path.sock"));
        assert_eq!(app.socket_path, PathBuf::from("/custom/path.sock"));
    }

    #[test]
    fn test_app_debug_format() {
        let app = App::new(PathBuf::from("/tmp/debug.sock"));
        let debug = format!("{:?}", app);
        assert!(debug.contains("should_quit"));
        assert!(debug.contains("socket_path"));
        assert!(debug.contains("tick_count"));
        assert!(debug.contains("sessions"));
        assert!(debug.contains("selected_index"));
    }

    // --- init_selection tests ---

    #[test]
    fn test_init_selection_empty() {
        let mut app = App::new(PathBuf::from("/tmp/test.sock"));
        app.init_selection();
        assert_eq!(app.selected_index, None);
    }

    #[test]
    fn test_init_selection_with_sessions() {
        let app = make_app_with_sessions(3);
        assert_eq!(app.selected_index, Some(0));
    }

    // --- select_next tests ---

    #[test]
    fn test_select_next_moves_down() {
        let mut app = make_app_with_sessions(3);
        app.select_next();
        assert_eq!(app.selected_index, Some(1));
        app.select_next();
        assert_eq!(app.selected_index, Some(2));
    }

    #[test]
    fn test_select_next_clamps_at_end() {
        let mut app = make_app_with_sessions(3);
        app.selected_index = Some(2);
        app.select_next();
        assert_eq!(app.selected_index, Some(2));
    }

    #[test]
    fn test_select_next_empty_sessions() {
        let mut app = App::new(PathBuf::from("/tmp/test.sock"));
        app.select_next();
        assert_eq!(app.selected_index, None);
    }

    // --- select_previous tests ---

    #[test]
    fn test_select_previous_moves_up() {
        let mut app = make_app_with_sessions(3);
        app.selected_index = Some(2);
        app.select_previous();
        assert_eq!(app.selected_index, Some(1));
    }

    #[test]
    fn test_select_previous_clamps_at_zero() {
        let mut app = make_app_with_sessions(3);
        app.select_previous();
        assert_eq!(app.selected_index, Some(0));
    }

    #[test]
    fn test_select_previous_empty_sessions() {
        let mut app = App::new(PathBuf::from("/tmp/test.sock"));
        app.select_previous();
        assert_eq!(app.selected_index, None);
    }

    // --- selected_session tests ---

    #[test]
    fn test_selected_session_returns_correct() {
        let app = make_app_with_sessions(3);
        let session = app.selected_session().expect("should have selected session");
        assert_eq!(session.id, "session-0");
    }

    #[test]
    fn test_selected_session_none_when_empty() {
        let app = App::new(PathBuf::from("/tmp/test.sock"));
        assert!(app.selected_session().is_none());
    }

    #[test]
    fn test_selected_session_after_navigation() {
        let mut app = make_app_with_sessions(5);
        app.select_next();
        app.select_next();
        let session = app.selected_session().expect("should have selected session");
        assert_eq!(session.id, "session-2");
    }

    // --- integration: multiple nav steps ---

    #[test]
    fn test_navigation_sequence() {
        let mut app = make_app_with_sessions(4);
        // Down to end
        app.select_next();
        app.select_next();
        app.select_next();
        assert_eq!(app.selected_index, Some(3));
        // Try going past end
        app.select_next();
        assert_eq!(app.selected_index, Some(3));
        // Back up to start
        app.select_previous();
        app.select_previous();
        app.select_previous();
        assert_eq!(app.selected_index, Some(0));
        // Try going past start
        app.select_previous();
        assert_eq!(app.selected_index, Some(0));
    }

    #[test]
    fn test_single_session_navigation() {
        let mut app = make_app_with_sessions(1);
        assert_eq!(app.selected_index, Some(0));
        app.select_next();
        assert_eq!(app.selected_index, Some(0));
        app.select_previous();
        assert_eq!(app.selected_index, Some(0));
    }
}
