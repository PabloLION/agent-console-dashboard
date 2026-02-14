//! Application state and main event loop for the TUI.
//!
//! Manages terminal setup/teardown, panic hooks, and the core render loop.

mod update;

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

/// Passive refresh interval for elapsed-time recalculation.
///
/// This throttles background elapsed-time updates to 1 second, reducing CPU
/// usage at scale (100 sessions × 100 TUIs = 10,000 calculations per tick).
/// User input events (keyboard, mouse) bypass this throttle and render immediately.
const ELAPSED_TIME_REFRESH_INTERVAL: Duration = Duration::from_secs(1);

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
    /// Temporary status message shown in footer, with expiry time.
    pub status_message: Option<(String, Instant)>,
    /// Last time elapsed-time rendering occurred (for throttling passive updates).
    last_elapsed_render: Instant,
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
            status_message: None,
            last_elapsed_render: Instant::now(),
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
    ///
    /// Pipes the full SessionSnapshot as JSON to the hook's stdin, following the
    /// same pattern as Claude Code hooks.
    fn execute_double_click_hook(&self, session: &Session) {
        use crate::SessionSnapshot;
        use std::io::Write;

        if let Some(ref hook) = self.double_click_hook {
            let cmd = substitute_hook_placeholders(hook, session);
            tracing::debug!("executing double-click hook: {}", cmd);

            // Convert Session to SessionSnapshot and serialize to JSON
            let snapshot: SessionSnapshot = session.into();
            let json_payload = match serde_json::to_string(&snapshot) {
                Ok(json) => json,
                Err(e) => {
                    tracing::warn!("failed to serialize SessionSnapshot: {}", e);
                    return;
                }
            };

            match std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                Ok(mut child) => {
                    tracing::debug!("double-click hook spawned: {}", cmd);
                    if let Some(ref mut stdin) = child.stdin {
                        if let Err(e) = stdin.write_all(json_payload.as_bytes()) {
                            tracing::warn!("failed to write to hook stdin: {}", e);
                        }
                    }
                }
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
                        if self.double_click_hook.is_some() {
                            // Fire the hook and show confirmation
                            if let Some(session) = self.sessions.get(idx) {
                                let session_clone = session.clone();
                                self.execute_double_click_hook(&session_clone);
                            }
                            self.status_message = Some((
                                "Hook executed".to_string(),
                                Instant::now() + Duration::from_secs(2),
                            ));
                        } else {
                            // No hook configured — tell the user where to set it
                            self.status_message = Some((
                                "Set tui.double_click_hook in config to enable double-click action"
                                    .to_string(),
                                Instant::now() + Duration::from_secs(3),
                            ));
                        }
                        return Action::None;
                    }
                    // Single click: select + open inline detail
                    self.open_detail(idx);
                } else {
                    // Click on header or outside list → clear selection
                    self.selected_index = None;
                    self.view = View::Dashboard;
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

    /// Clears the status message if its expiry time has passed.
    pub fn expire_status_message(&mut self) {
        if let Some((_, expiry)) = &self.status_message {
            if Instant::now() >= *expiry {
                self.status_message = None;
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

            // Handle events first to determine if we should render
            let event = event_handler.next(&mut reader).await?;
            let should_render = match event {
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
                    true // Input events always render immediately
                }
                Event::Mouse(mouse) => {
                    self.handle_mouse_event(mouse);
                    true // Input events always render immediately
                }
                Event::Tick => {
                    self.tick_count += 1;
                    self.expire_status_message();
                    // Passive tick: only render if interval has elapsed
                    self.last_elapsed_render.elapsed() >= ELAPSED_TIME_REFRESH_INTERVAL
                }
                Event::Resize(_, _) => {
                    true // Resize always renders immediately
                }
            };

            // Render only when needed (input events or throttled passive tick)
            if should_render {
                terminal.draw(|frame| {
                    render_dashboard(frame, self);
                })?;
                self.last_elapsed_render = Instant::now();
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
mod tests;
