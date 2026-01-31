//! Application state and main event loop for the TUI.
//!
//! Manages terminal setup/teardown, panic hooks, and the core render loop.

use crate::tui::event::{handle_key_event, Action, Event, EventHandler};
use crate::tui::ui::render_dashboard;
use crate::tui::views::detail::render_detail;
use crate::{AgentType, Session, Status};
use crossterm::{
    event::EventStream,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::{CrosstermBackend, Terminal};
use std::io::{self, stdout};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

/// Active view state for the TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    /// Main dashboard showing session list.
    Dashboard,
    /// Detail modal overlay for a specific session.
    Detail {
        /// Index of the session being viewed.
        session_index: usize,
        /// Scroll offset for history entries.
        history_scroll: usize,
    },
}

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
    /// Current active view.
    pub view: View,
    /// Active layout preset index (1=default, 2=compact).
    pub layout_preset: u8,
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
            view: View::Dashboard,
            layout_preset: 1,
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

    /// Opens the detail view for the session at `index`.
    pub fn open_detail(&mut self, index: usize) {
        if index < self.sessions.len() {
            self.view = View::Detail {
                session_index: index,
                history_scroll: 0,
            };
        }
    }

    /// Closes any overlay and returns to the dashboard view.
    pub fn close_detail(&mut self) {
        self.view = View::Dashboard;
    }

    /// Scrolls the detail history down by one entry.
    pub fn scroll_history_down(&mut self) {
        if let View::Detail {
            session_index,
            ref mut history_scroll,
        } = self.view
        {
            if let Some(session) = self.sessions.get(session_index) {
                let max_scroll = session.history.len().saturating_sub(5);
                if *history_scroll < max_scroll {
                    *history_scroll += 1;
                }
            }
        }
    }

    /// Scrolls the detail history up by one entry.
    pub fn scroll_history_up(&mut self) {
        if let View::Detail {
            ref mut history_scroll,
            ..
        } = self.view
        {
            *history_scroll = history_scroll.saturating_sub(1);
        }
    }

    /// Applies a daemon update message to the session list.
    fn apply_update(&mut self, session_id: &str, status: Status) {
        if let Some(session) = self.sessions.iter_mut().find(|s| s.id == session_id) {
            if session.status != status {
                session.history.push(crate::StateTransition {
                    timestamp: Instant::now(),
                    from: session.status,
                    to: status,
                    duration: session.since.elapsed(),
                });
                session.status = status;
                session.since = Instant::now();
            }
        } else {
            let session = Session::new(
                session_id.to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("unknown"),
            );
            self.sessions.push(session);
            if self.selected_index.is_none() {
                self.selected_index = Some(0);
            }
        }
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

        // Connect to daemon and subscribe to updates
        let (update_tx, mut update_rx) = mpsc::channel::<(String, Status)>(64);
        let socket_path = self.socket_path.clone();
        tokio::spawn(async move {
            if let Err(e) = subscribe_to_daemon(&socket_path, update_tx).await {
                tracing::warn!("daemon subscription failed: {}", e);
            }
        });

        loop {
            // Drain daemon updates before rendering
            while let Ok((session_id, status)) = update_rx.try_recv() {
                self.apply_update(&session_id, status);
            }

            // Render
            let now = Instant::now();
            terminal.draw(|frame| {
                render_dashboard(frame, self);
                // Render detail overlay on top if active
                if let View::Detail {
                    session_index,
                    history_scroll,
                } = self.view
                {
                    if let Some(session) = self.sessions.get(session_index) {
                        render_detail(frame, session, frame.area(), history_scroll, now);
                    }
                }
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
                            self.open_detail(idx);
                        }
                        Action::Resurrect(id) => {
                            tracing::debug!("resurrect session {id}");
                            // TODO: send RESURRECT IPC command to daemon
                        }
                        Action::Remove(id) => {
                            tracing::debug!("remove session {id}");
                            // TODO: show confirmation, then send REMOVE IPC command
                        }
                        Action::SwitchLayout(preset) => {
                            tracing::debug!("switch to layout preset {preset}");
                            if (1..=2).contains(&preset) {
                                self.layout_preset = preset;
                            }
                        }
                        Action::Back => {
                            self.close_detail();
                        }
                        Action::ScrollHistoryDown => {
                            self.scroll_history_down();
                        }
                        Action::ScrollHistoryUp => {
                            self.scroll_history_up();
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

/// Connects to the daemon via Unix socket, sends LIST to get initial state,
/// then SUB to receive live updates. Sends parsed updates through the channel.
async fn subscribe_to_daemon(
    socket_path: &PathBuf,
    tx: mpsc::Sender<(String, Status)>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::client::connect_with_auto_start;

    let client = connect_with_auto_start(socket_path).await?;
    let stream = client.into_stream();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Fetch initial session list
    writer.write_all(b"LIST\n").await?;
    writer.flush().await?;

    let mut line = String::new();
    reader.read_line(&mut line).await?; // "OK\n" header
    if line.trim() == "OK" {
        // Read session lines until empty line or next command
        loop {
            line.clear();
            // Use a short timeout to detect end of LIST response
            match tokio::time::timeout(Duration::from_millis(100), reader.read_line(&mut line)).await
            {
                Ok(Ok(0)) => break,
                Ok(Ok(_)) => {
                    let parts: Vec<&str> = line.trim().split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(status) = parts[1].parse::<Status>() {
                            let _ = tx.send((parts[0].to_string(), status)).await;
                        }
                    }
                }
                _ => break,
            }
        }
    }

    // Now subscribe for live updates — need a new connection since LIST consumed the first
    let client = connect_with_auto_start(socket_path).await?;
    let stream = client.into_stream();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    writer.write_all(b"SUB\n").await?;
    writer.flush().await?;

    line.clear();
    reader.read_line(&mut line).await?; // "OK subscribed\n"

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).await?;
        if bytes == 0 {
            break;
        }
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        // UPDATE <session_id> <status> <elapsed>
        if parts.len() >= 3 && parts[0] == "UPDATE" {
            if let Ok(status) = parts[2].parse::<Status>() {
                if tx.send((parts[1].to_string(), status)).await.is_err() {
                    break; // receiver dropped
                }
            }
        }
    }

    Ok(())
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
        assert_eq!(app.view, View::Dashboard);
        assert_eq!(app.layout_preset, 1);
    }

    #[test]
    fn test_app_default_state() {
        let app = App::new(PathBuf::from("/tmp/agent-console.sock"));
        assert!(!app.should_quit);
        assert_eq!(app.tick_count, 0);
        assert!(app.sessions.is_empty());
        assert!(app.selected_index.is_none());
        assert_eq!(app.view, View::Dashboard);
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
        assert!(debug.contains("view"));
        assert!(debug.contains("layout_preset"));
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

    // --- View state tests ---

    #[test]
    fn test_open_detail_sets_view() {
        let mut app = make_app_with_sessions(3);
        app.open_detail(1);
        assert_eq!(
            app.view,
            View::Detail {
                session_index: 1,
                history_scroll: 0,
            }
        );
    }

    #[test]
    fn test_open_detail_out_of_bounds_no_change() {
        let mut app = make_app_with_sessions(3);
        app.open_detail(5);
        assert_eq!(app.view, View::Dashboard);
    }

    #[test]
    fn test_close_detail_returns_to_dashboard() {
        let mut app = make_app_with_sessions(3);
        app.open_detail(0);
        app.close_detail();
        assert_eq!(app.view, View::Dashboard);
    }

    #[test]
    fn test_scroll_history_down() {
        let mut app = make_app_with_sessions(1);
        // Add enough history entries
        for _ in 0..10 {
            app.sessions[0].history.push(crate::StateTransition {
                timestamp: std::time::Instant::now(),
                from: crate::Status::Working,
                to: crate::Status::Attention,
                duration: std::time::Duration::from_secs(1),
            });
        }
        app.open_detail(0);
        app.scroll_history_down();
        assert_eq!(
            app.view,
            View::Detail {
                session_index: 0,
                history_scroll: 1,
            }
        );
    }

    #[test]
    fn test_scroll_history_up() {
        let mut app = make_app_with_sessions(1);
        for _ in 0..10 {
            app.sessions[0].history.push(crate::StateTransition {
                timestamp: std::time::Instant::now(),
                from: crate::Status::Working,
                to: crate::Status::Attention,
                duration: std::time::Duration::from_secs(1),
            });
        }
        app.view = View::Detail {
            session_index: 0,
            history_scroll: 3,
        };
        app.scroll_history_up();
        assert_eq!(
            app.view,
            View::Detail {
                session_index: 0,
                history_scroll: 2,
            }
        );
    }

    #[test]
    fn test_scroll_history_up_clamps_at_zero() {
        let mut app = make_app_with_sessions(1);
        app.open_detail(0);
        app.scroll_history_up();
        assert_eq!(
            app.view,
            View::Detail {
                session_index: 0,
                history_scroll: 0,
            }
        );
    }

    #[test]
    fn test_layout_preset_default() {
        let app = App::new(PathBuf::from("/tmp/test.sock"));
        assert_eq!(app.layout_preset, 1);
    }
}
