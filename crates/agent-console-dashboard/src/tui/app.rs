//! Application state and main event loop for the TUI.
//!
//! Manages terminal setup/teardown, panic hooks, and the core render loop.

use crate::tui::event::{handle_key_event, Action, Event, EventHandler};
use crate::tui::subscription::{subscribe_to_daemon, DaemonMessage};
use crate::tui::ui::render_dashboard;
use crate::{AgentType, Session, Status};
use claude_usage::UsageData;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, EventStream},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::{CrosstermBackend, Terminal};
use std::io::{self, stdout};
use std::path::PathBuf;
use std::time::{Duration, Instant};
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
    /// Latest API usage data from the daemon, if available.
    pub usage: Option<UsageData>,
    /// Last click time and position for double-click detection.
    last_click: Option<(Instant, u16, u16)>,
    /// Shell command to execute on double-click, with placeholder support.
    ///
    /// Loaded from `tui.double_click_hook` in config. `None` means no hook
    /// configured (empty string in config is treated as no hook).
    pub double_click_hook: Option<String>,
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
            usage: None,
            last_click: None,
            double_click_hook: None,
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

    /// Executes the double-click hook for the given session, if configured.
    ///
    /// Substitutes `{session_id}`, `{working_dir}`, and `{status}` placeholders
    /// in the hook command, then spawns it via `sh -c` in fire-and-forget mode.
    /// The child process is detached (stdout/stderr piped to null).
    fn execute_double_click_hook(&self, session: &Session) {
        if let Some(ref hook) = self.double_click_hook {
            let cmd = substitute_hook_placeholders(hook, session);
            tracing::debug!("executing double-click hook: {}", cmd);
            match std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                Ok(_) => tracing::debug!("double-click hook spawned: {}", cmd),
                Err(e) => tracing::warn!("double-click hook failed: {}: {}", cmd, e),
            }
        }
    }

    /// Calculates which session index was clicked based on mouse row coordinate.
    ///
    /// Returns None if the click was outside the session list area.
    /// Accounts for header (1 line) and block borders (1 line at top).
    fn calculate_clicked_session(&self, row: u16) -> Option<usize> {
        // Header takes 1 line, block border takes 1 line
        // Session list starts at row 2
        if row < 2 {
            return None;
        }
        let list_row = (row - 2) as usize;
        if list_row < self.sessions.len() {
            Some(list_row)
        } else {
            None
        }
    }

    /// Handles a mouse event and returns the appropriate action.
    ///
    /// Processes mouse events in both Dashboard and Detail views since the
    /// detail panel is rendered inline (not as a modal overlay). Left click
    /// selects a session and immediately opens the inline detail panel.
    /// Double-click fires a configurable hook (see `double_click_hook`).
    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Action {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let now = Instant::now();
                let is_double_click = if let Some((last_time, last_row, last_col)) = self.last_click
                {
                    now.duration_since(last_time) < Duration::from_millis(500)
                        && mouse.row == last_row
                        && mouse.column == last_col
                } else {
                    false
                };

                if let Some(idx) = self.calculate_clicked_session(mouse.row) {
                    self.selected_index = Some(idx);
                    if is_double_click {
                        self.last_click = None;
                        // Double-click: fire hook if configured, otherwise no-op
                        if let Some(session) = self.sessions.get(idx) {
                            let session_clone = session.clone();
                            self.execute_double_click_hook(&session_clone);
                        }
                        return Action::None;
                    }
                    // Single click: select + open inline detail
                    self.open_detail(idx);
                }

                self.last_click = Some((now, mouse.row, mouse.column));
                Action::None
            }
            MouseEventKind::ScrollDown => {
                if matches!(self.view, View::Detail { .. }) {
                    self.scroll_history_down();
                } else {
                    self.select_next();
                }
                Action::None
            }
            MouseEventKind::ScrollUp => {
                if matches!(self.view, View::Detail { .. }) {
                    self.scroll_history_up();
                } else {
                    self.select_previous();
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    /// Applies a daemon update message (full `SessionSnapshot`) to the session list.
    ///
    /// `elapsed_seconds` is the time since the session entered its current
    /// status, as reported by the daemon. We backdate `session.since` by
    /// subtracting this duration from `Instant::now()` so elapsed time
    /// displays correctly even though `Instant` cannot survive IPC.
    fn apply_update(&mut self, info: &crate::SessionSnapshot) {
        let status: Status = info.status.parse().unwrap_or(Status::Working);
        let backdated_since = Instant::now()
            .checked_sub(Duration::from_secs(info.elapsed_seconds))
            .unwrap_or_else(Instant::now);
        let backdated_activity = Instant::now()
            .checked_sub(Duration::from_secs(info.idle_seconds))
            .unwrap_or_else(Instant::now);
        let working_dir = info.working_dir.as_ref().map(PathBuf::from);

        if let Some(session) = self
            .sessions
            .iter_mut()
            .find(|s| s.session_id == info.session_id)
        {
            // Update working_dir from daemon if Some
            if working_dir.is_some() {
                session.working_dir = working_dir.clone();
            }
            if session.status != status {
                session.history.push(crate::StateTransition {
                    timestamp: Instant::now(),
                    from: session.status,
                    to: status,
                    duration: session.since.elapsed(),
                });
                session.status = status;
                session.since = backdated_since;
            }
            session.last_activity = backdated_activity;
            session.closed = info.closed;
        } else {
            let mut session = Session::new(
                info.session_id.clone(),
                AgentType::ClaudeCode,
                working_dir.clone(),
            );
            session.status = status;
            session.since = backdated_since;
            session.last_activity = backdated_activity;
            session.closed = info.closed;
            // Reconstruct history from wire StatusChange entries
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let entries = &info.history;
            for i in 0..entries.len() {
                let to = entries[i]
                    .status
                    .parse::<Status>()
                    .unwrap_or(Status::Working);
                let from = if i > 0 {
                    entries[i - 1]
                        .status
                        .parse::<Status>()
                        .unwrap_or(Status::Working)
                } else {
                    Status::Working
                };
                let duration = if i > 0 {
                    Duration::from_secs(entries[i].at_secs.saturating_sub(entries[i - 1].at_secs))
                } else {
                    Duration::from_secs(0)
                };
                // Approximate Instant from unix timestamp
                let secs_ago = now_secs.saturating_sub(entries[i].at_secs);
                let timestamp = Instant::now()
                    .checked_sub(Duration::from_secs(secs_ago))
                    .unwrap_or_else(Instant::now);
                session.history.push(crate::StateTransition {
                    timestamp,
                    from,
                    to,
                    duration,
                });
            }
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
        let (update_tx, mut update_rx) = mpsc::channel::<DaemonMessage>(64);
        let socket_path = self.socket_path.clone();
        tokio::spawn(async move {
            if let Err(e) = subscribe_to_daemon(&socket_path, update_tx).await {
                tracing::warn!("daemon subscription failed: {}", e);
            }
        });

        loop {
            // Drain daemon updates before rendering
            while let Ok(msg) = update_rx.try_recv() {
                match msg {
                    DaemonMessage::SessionUpdate(info) => self.apply_update(&info),
                    DaemonMessage::UsageUpdate(data) => {
                        self.usage = Some(data);
                    }
                }
            }

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
                Event::Mouse(mouse) => {
                    self.handle_mouse_event(mouse);
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
    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    Ok(())
}

/// Restores the terminal to its original state.
fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

/// Substitutes placeholders in a hook command template with session values.
///
/// Supported placeholders:
/// - `{session_id}` — replaced with `session.session_id`
/// - `{working_dir}` — replaced with `session.working_dir` display string
/// - `{status}` — replaced with `session.status` display string
pub fn substitute_hook_placeholders(template: &str, session: &Session) -> String {
    let working_dir_str = session
        .working_dir
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<none>".to_string());
    template
        .replace("{session_id}", &session.session_id)
        .replace("{working_dir}", &working_dir_str)
        .replace("{status}", &session.status.to_string())
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
                Some(PathBuf::from(format!("/home/user/project-{}", i))),
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
        assert!(app.usage.is_none());
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
        let session = app
            .selected_session()
            .expect("should have selected session");
        assert_eq!(session.session_id, "session-0");
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
        let session = app
            .selected_session()
            .expect("should have selected session");
        assert_eq!(session.session_id, "session-2");
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

    // --- App usage field tests ---

    #[test]
    fn test_app_usage_starts_none() {
        let app = App::new(PathBuf::from("/tmp/test.sock"));
        assert!(app.usage.is_none());
    }

    // --- Mouse event handling tests ---

    fn make_mouse_event(kind: MouseEventKind, row: u16, column: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column,
            row,
            modifiers: crossterm::event::KeyModifiers::NONE,
        }
    }

    #[test]
    fn test_calculate_clicked_session_valid_row() {
        let app = make_app_with_sessions(5);
        // Header at row 0, border at row 1, first session at row 2
        assert_eq!(app.calculate_clicked_session(2), Some(0));
        assert_eq!(app.calculate_clicked_session(3), Some(1));
        assert_eq!(app.calculate_clicked_session(6), Some(4));
    }

    #[test]
    fn test_calculate_clicked_session_header_returns_none() {
        let app = make_app_with_sessions(5);
        assert_eq!(app.calculate_clicked_session(0), None);
        assert_eq!(app.calculate_clicked_session(1), None);
    }

    #[test]
    fn test_calculate_clicked_session_out_of_bounds() {
        let app = make_app_with_sessions(3);
        // Sessions at rows 2, 3, 4 (indices 0, 1, 2)
        assert_eq!(app.calculate_clicked_session(5), None);
        assert_eq!(app.calculate_clicked_session(10), None);
    }

    #[test]
    fn test_mouse_left_click_selects_and_opens_detail() {
        let mut app = make_app_with_sessions(5);
        app.selected_index = Some(0);
        let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 4, 10);
        let action = app.handle_mouse_event(mouse);
        assert_eq!(action, Action::None);
        assert_eq!(app.selected_index, Some(2));
        // Single click should also open inline detail
        assert_eq!(
            app.view,
            View::Detail {
                session_index: 2,
                history_scroll: 0,
            }
        );
    }

    #[test]
    fn test_mouse_left_click_outside_list_no_change() {
        let mut app = make_app_with_sessions(3);
        app.selected_index = Some(1);
        let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 0, 10);
        let action = app.handle_mouse_event(mouse);
        assert_eq!(action, Action::None);
        assert_eq!(app.selected_index, Some(1));
    }

    #[test]
    fn test_mouse_double_click_fires_hook_returns_none() {
        let mut app = make_app_with_sessions(3);
        // First click: selects and opens inline detail
        let mouse1 = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 3, 10);
        let action1 = app.handle_mouse_event(mouse1);
        assert_eq!(action1, Action::None);
        assert_eq!(app.selected_index, Some(1));
        // Single click opens inline detail
        assert_eq!(
            app.view,
            View::Detail {
                session_index: 1,
                history_scroll: 0,
            }
        );

        // Second click in quick succession at same position (double-click)
        // Double-click returns None (hook fires internally, not via Action)
        let mouse2 = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 3, 10);
        let action2 = app.handle_mouse_event(mouse2);
        assert_eq!(action2, Action::None);
        // last_click should be cleared after double-click
        assert!(app.last_click.is_none());
    }

    #[test]
    fn test_mouse_double_click_different_position_no_detail() {
        let mut app = make_app_with_sessions(5);
        // First click
        let mouse1 = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 3, 10);
        app.handle_mouse_event(mouse1);

        // Second click at different row
        let mouse2 = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 4, 10);
        let action2 = app.handle_mouse_event(mouse2);
        assert_eq!(action2, Action::None);
        assert_eq!(app.selected_index, Some(2));
    }

    #[test]
    fn test_mouse_scroll_down_selects_next() {
        let mut app = make_app_with_sessions(5);
        app.selected_index = Some(1);
        let mouse = make_mouse_event(MouseEventKind::ScrollDown, 5, 10);
        let action = app.handle_mouse_event(mouse);
        assert_eq!(action, Action::None);
        assert_eq!(app.selected_index, Some(2));
    }

    #[test]
    fn test_mouse_scroll_up_selects_previous() {
        let mut app = make_app_with_sessions(5);
        app.selected_index = Some(2);
        let mouse = make_mouse_event(MouseEventKind::ScrollUp, 5, 10);
        let action = app.handle_mouse_event(mouse);
        assert_eq!(action, Action::None);
        assert_eq!(app.selected_index, Some(1));
    }

    #[test]
    fn test_mouse_scroll_at_boundaries() {
        let mut app = make_app_with_sessions(3);
        // Scroll down at end
        app.selected_index = Some(2);
        let mouse_down = make_mouse_event(MouseEventKind::ScrollDown, 5, 10);
        app.handle_mouse_event(mouse_down);
        assert_eq!(app.selected_index, Some(2));

        // Scroll up at start
        app.selected_index = Some(0);
        let mouse_up = make_mouse_event(MouseEventKind::ScrollUp, 5, 10);
        app.handle_mouse_event(mouse_up);
        assert_eq!(app.selected_index, Some(0));
    }

    #[test]
    fn test_mouse_click_in_detail_view_reselects() {
        let mut app = make_app_with_sessions(3);
        app.open_detail(0);
        app.selected_index = Some(0);

        // Click on a different session row while detail is open
        let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 3, 10);
        let action = app.handle_mouse_event(mouse);
        assert_eq!(action, Action::None);
        // Should have selected new session and opened detail for it
        assert_eq!(app.selected_index, Some(1));
        assert_eq!(
            app.view,
            View::Detail {
                session_index: 1,
                history_scroll: 0,
            }
        );
    }

    #[test]
    fn test_mouse_scroll_in_detail_view_scrolls_history() {
        let mut app = make_app_with_sessions(1);
        for _ in 0..10 {
            app.sessions[0].history.push(crate::StateTransition {
                timestamp: std::time::Instant::now(),
                from: crate::Status::Working,
                to: crate::Status::Attention,
                duration: std::time::Duration::from_secs(1),
            });
        }
        app.open_detail(0);

        // Scroll down should scroll history, not select next session
        let scroll = make_mouse_event(MouseEventKind::ScrollDown, 5, 10);
        let action = app.handle_mouse_event(scroll);
        assert_eq!(action, Action::None);
        assert_eq!(
            app.view,
            View::Detail {
                session_index: 0,
                history_scroll: 1,
            }
        );
    }

    #[test]
    fn test_mouse_right_click_ignored() {
        let mut app = make_app_with_sessions(3);
        app.selected_index = Some(0);
        let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Right), 3, 10);
        let action = app.handle_mouse_event(mouse);
        assert_eq!(action, Action::None);
        assert_eq!(app.selected_index, Some(0));
    }

    #[test]
    fn test_last_click_initialized_to_none() {
        let app = App::new(PathBuf::from("/tmp/test.sock"));
        assert!(app.last_click.is_none());
    }

    #[test]
    fn test_double_click_hook_default_none() {
        let app = App::new(PathBuf::from("/tmp/test.sock"));
        assert!(app.double_click_hook.is_none());
    }

    // --- substitute_hook_placeholders tests ---

    #[test]
    fn test_substitute_hook_all_placeholders() {
        let session = Session::new(
            "sess-123".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/home/user/project")),
        );
        let result = substitute_hook_placeholders("open {working_dir} --id={session_id}", &session);
        assert_eq!(result, "open /home/user/project --id=sess-123");
    }

    #[test]
    fn test_substitute_hook_status_placeholder() {
        let mut session = Session::new(
            "sess-456".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
        );
        session.status = crate::Status::Attention;
        let result = substitute_hook_placeholders("echo {status}", &session);
        assert_eq!(result, "echo attention");
    }

    #[test]
    fn test_substitute_hook_no_placeholders() {
        let session = Session::new(
            "sess-789".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
        );
        let result = substitute_hook_placeholders("echo hello", &session);
        assert_eq!(result, "echo hello");
    }

    #[test]
    fn test_substitute_hook_repeated_placeholders() {
        let session = Session::new(
            "abc".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/x")),
        );
        let result = substitute_hook_placeholders("{session_id} and {session_id}", &session);
        assert_eq!(result, "abc and abc");
    }

    #[test]
    fn test_substitute_hook_empty_template() {
        let session = Session::new(
            "s".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/")),
        );
        let result = substitute_hook_placeholders("", &session);
        assert_eq!(result, "");
    }
}
