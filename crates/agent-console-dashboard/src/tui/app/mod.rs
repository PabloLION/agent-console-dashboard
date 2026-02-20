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
use ratatui::layout::Rect;
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

/// Terminal height threshold for TwoLine layout mode.
///
/// When terminal height is less than this value, TwoLine mode is automatically
/// selected. Otherwise, Large mode is used.
pub const TWO_LINE_LAYOUT_HEIGHT_THRESHOLD: u16 = 5;

/// Layout mode for the TUI rendering.
///
/// Controls how the dashboard renders based on available screen space:
/// - `Large`: Full multi-line layout with header, session list, detail panel, footer
/// - `TwoLine`: Compact layout with session chips (line 1) and API usage (line 2)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// Full dashboard layout with session list and detail panel.
    Large,
    /// Compact two-line layout for narrow terminals.
    TwoLine,
}

/// Active view state for the TUI.
///
/// Deprecated: detail panel is now always visible. This enum is kept for
/// backward compatibility but only Dashboard variant is used. The history
/// scroll offset is tracked directly in App.history_scroll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    /// Main dashboard showing session list with always-visible detail panel.
    Dashboard,
    /// Detail modal overlay (deprecated, not used).
    Detail {
        /// Index of the session being viewed.
        session_index: usize,
        /// Scroll offset for history entries.
        history_scroll: usize,
    },
}

/// Target of a mouse click in TwoLine layout mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClickTarget {
    /// Click on a session chip at the given global index.
    Chip(usize),
    /// Click on left overflow indicator (`<- N+`).
    LeftOverflow,
    /// Click on right overflow indicator (`N+ ->`).
    RightOverflow,
    /// Click outside any interactive element.
    None,
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
    /// Current active view (deprecated, always Dashboard now).
    pub view: View,
    /// Scroll offset for history entries in the detail panel.
    pub history_scroll: usize,
    /// Active layout preset index (1=default, 2=compact).
    pub layout_preset: u8,
    /// Latest API usage data from the daemon, if available.
    pub usage: Option<UsageData>,
    /// Last click time and position for double-click detection.
    last_click: Option<(Instant, u16, u16)>,
    /// Shell command to execute on double-click/Enter for non-closed sessions.
    ///
    /// Loaded from `tui.activate_hook` in config. `None` means no hook configured.
    pub activate_hook: Option<String>,
    /// Shell command to execute on double-click/Enter/'r' for closed sessions.
    ///
    /// Loaded from `tui.reopen_hook` in config. `None` means no hook configured.
    pub reopen_hook: Option<String>,
    /// Temporary status message shown in footer, with expiry time.
    pub status_message: Option<(String, Instant)>,
    /// Last time elapsed-time rendering occurred (for throttling passive updates).
    last_elapsed_render: Instant,
    /// Inner area of the session list widget (excluding block borders).
    ///
    /// Updated during each render pass. Used by mouse click detection to accurately
    /// map click coordinates to session indices. None if the list hasn't been rendered yet.
    pub session_list_inner_area: Option<Rect>,
    /// Current layout mode (Large or TwoLine).
    ///
    /// Auto-detected based on terminal height during render. TwoLine mode is used
    /// when height < TWO_LINE_LAYOUT_HEIGHT_THRESHOLD.
    pub layout_mode: LayoutMode,
    /// Layout mode override from CLI flag.
    ///
    /// When Some(mode), forces that layout mode and skips auto-detection.
    /// When None, auto-detects from terminal height (existing behavior).
    pub layout_mode_override: Option<LayoutMode>,
    /// Horizontal scroll offset for compact two-line layout.
    ///
    /// Tracks which session chip is leftmost in the viewport. Only used in TwoLine
    /// layout mode for horizontal pagination. Zero-indexed into the sessions list.
    pub compact_scroll_offset: usize,
    /// Terminal width (updated during each render pass).
    ///
    /// Used by mouse click detection in TwoLine mode to calculate chip positions.
    pub terminal_width: u16,
}

impl App {
    /// Creates a new App with the given socket path and optional layout mode override.
    ///
    /// # Arguments
    ///
    /// * `socket_path` - Path to the daemon socket for IPC
    /// * `layout_mode_override` - Optional layout mode to force (skips auto-detection)
    ///
    /// When `layout_mode_override` is `None`, the layout mode is auto-detected from
    /// terminal height during render. When `Some(mode)`, that mode is forced.
    pub fn new(socket_path: PathBuf, layout_mode_override: Option<LayoutMode>) -> Self {
        // If override is set, use it as initial layout_mode
        let initial_mode = layout_mode_override.unwrap_or(LayoutMode::Large);
        Self {
            should_quit: false,
            socket_path,
            tick_count: 0,
            sessions: Vec::new(),
            selected_index: None,
            view: View::Dashboard,
            history_scroll: 0,
            layout_preset: 1,
            usage: None,
            last_click: None,
            activate_hook: None,
            reopen_hook: None,
            status_message: None,
            last_elapsed_render: Instant::now(),
            session_list_inner_area: None,
            layout_mode: initial_mode,
            layout_mode_override,
            compact_scroll_offset: 0,
            terminal_width: 80, // Default, updated during render
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
    ///
    /// Resets history scroll when selection changes.
    pub fn select_next(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        let last = self.sessions.len().saturating_sub(1);
        let new_idx = self.selected_index.map_or(0, |i| (i + 1).min(last));
        if self.selected_index != Some(new_idx) {
            self.history_scroll = 0;
        }
        self.selected_index = Some(new_idx);
    }

    /// Moves the selection up by one, clamped to index 0.
    ///
    /// Resets history scroll when selection changes.
    pub fn select_previous(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        let new_idx = self.selected_index.map_or(0, |i| i.saturating_sub(1));
        if self.selected_index != Some(new_idx) {
            self.history_scroll = 0;
        }
        self.selected_index = Some(new_idx);
    }

    /// Scrolls the compact layout viewport left by one chip.
    ///
    /// Only applicable in TwoLine layout mode. Decrements compact_scroll_offset
    /// down to zero (left boundary).
    pub fn scroll_compact_left(&mut self) {
        self.compact_scroll_offset = self.compact_scroll_offset.saturating_sub(1);
    }

    /// Scrolls the compact layout viewport right by one chip.
    ///
    /// Only applicable in TwoLine layout mode. Increments compact_scroll_offset
    /// up to the maximum valid offset (sessions.len() - 1).
    pub fn scroll_compact_right(&mut self) {
        if !self.sessions.is_empty() {
            let max_offset = self.sessions.len().saturating_sub(1);
            if self.compact_scroll_offset < max_offset {
                self.compact_scroll_offset += 1;
            }
        }
    }

    /// Ensures the selected session chip is visible in the compact layout viewport.
    ///
    /// Adjusts compact_scroll_offset to bring the selected session into view if needed.
    /// This is called automatically when selection changes via j/k or mouse click.
    ///
    /// # Arguments
    ///
    /// * `max_visible` - Maximum count of chips that fit in the current viewport
    pub fn ensure_selected_visible_compact(&mut self, max_visible: usize) {
        if let Some(idx) = self.selected_index {
            // If selected is before viewport, scroll left to show it
            if idx < self.compact_scroll_offset {
                self.compact_scroll_offset = idx;
            }
            // If selected is after viewport, scroll right to show it
            else if idx >= self.compact_scroll_offset + max_visible {
                self.compact_scroll_offset = (idx + 1).saturating_sub(max_visible);
            }
        }
    }

    /// Returns a reference to the currently selected session, if any.
    pub fn selected_session(&self) -> Option<&Session> {
        self.selected_index.and_then(|i| self.sessions.get(i))
    }

    /// Opens the detail view for the session at `index`.
    ///
    /// Deprecated: detail panel is always visible. This method is kept for
    /// backward compatibility but has no effect (detail is always shown).
    pub fn open_detail(&mut self, _index: usize) {
        // No-op: detail panel is always visible based on selected_index
    }

    /// Closes any overlay and returns to the dashboard view.
    ///
    /// Deprecated: detail panel is always visible. This method now just clears
    /// the selection (defocus).
    pub fn close_detail(&mut self) {
        self.selected_index = None;
        self.history_scroll = 0;
    }

    /// Scrolls the detail history down by one entry.
    pub fn scroll_history_down(&mut self) {
        if let Some(idx) = self.selected_index {
            if let Some(session) = self.sessions.get(idx) {
                let max_scroll = session.history.len().saturating_sub(5);
                if self.history_scroll < max_scroll {
                    self.history_scroll += 1;
                }
            }
        }
    }

    /// Scrolls the detail history up by one entry.
    pub fn scroll_history_up(&mut self) {
        self.history_scroll = self.history_scroll.saturating_sub(1);
    }

    /// Executes the appropriate hook for the given session based on its status.
    ///
    /// - Non-closed sessions → activate_hook
    /// - Closed sessions → reopen_hook
    ///
    /// Substitutes `{session_id}`, `{working_dir}`, and `{status}` placeholders
    /// in the hook command, then spawns it via `sh -c` in fire-and-forget mode.
    /// The child process is detached (stdout/stderr piped to null).
    ///
    /// Pipes the full SessionSnapshot as JSON to the hook's stdin, following the
    /// same pattern as Claude Code hooks.
    ///
    /// For closed sessions reopened via reopen_hook, the session status is updated
    /// locally to Attention (TUI-only, no IPC to daemon).
    pub fn execute_hook(&mut self, session_index: usize) {
        use crate::SessionSnapshot;
        use std::io::Write;

        let Some(session) = self.sessions.get(session_index) else {
            return;
        };

        let is_closed = session.status == Status::Closed;
        let hook = if is_closed {
            &self.reopen_hook
        } else {
            &self.activate_hook
        };

        let Some(ref hook_cmd) = hook else {
            // No hook configured — show hint message
            let config_path = crate::config::xdg::config_path();
            let key = if is_closed {
                "reopen_hook"
            } else {
                "activate_hook"
            };
            self.status_message = Some((
                format!(
                    "Set tui.{} in {} to enable this action",
                    key,
                    config_path.display()
                ),
                Instant::now() + Duration::from_secs(2),
            ));
            return;
        };

        let cmd = substitute_hook_placeholders(hook_cmd, session);
        let hook_type = if is_closed { "reopen" } else { "activate" };
        tracing::debug!("executing {} hook: {}", hook_type, cmd);

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
                tracing::debug!("{} hook spawned: {}", hook_type, cmd);
                if let Some(ref mut stdin) = child.stdin {
                    if let Err(e) = stdin.write_all(json_payload.as_bytes()) {
                        tracing::warn!("failed to write to hook stdin: {}", e);
                    }
                }

                // For closed sessions, update local status to Attention (no IPC)
                if is_closed {
                    if let Some(session) = self.sessions.get_mut(session_index) {
                        session.status = Status::Attention;
                        tracing::debug!("updated local session status to attention");
                    }
                }

                self.status_message = Some((
                    "Hook executed".to_string(),
                    Instant::now() + Duration::from_secs(2),
                ));
            }
            Err(e) => {
                tracing::warn!("{} hook failed: {}: {}", hook_type, cmd, e);
                self.status_message = Some((
                    format!("Hook failed: {}", e),
                    Instant::now() + Duration::from_secs(2),
                ));
            }
        }
    }

    /// Calculates which session index was clicked based on mouse row coordinate.
    ///
    /// Returns None if the click was outside the session list area.
    /// Uses the stored inner area from the last render pass to accurately map
    /// click coordinates to session indices across all layout modes (normal, debug, narrow).
    fn calculate_clicked_session(&self, row: u16) -> Option<usize> {
        let inner_area = self.session_list_inner_area?;

        // Check if click is within the inner area
        if row < inner_area.y || row >= inner_area.y + inner_area.height {
            return None;
        }

        // Calculate session index from row offset within inner area
        let list_row = (row - inner_area.y) as usize;
        if list_row < self.sessions.len() {
            Some(list_row)
        } else {
            None
        }
    }

    /// Handles a mouse event and returns the appropriate action.
    ///
    /// Processes mouse events for session list interaction. Left click focuses
    /// a session (updating the always-visible detail panel). Double-click focuses
    /// and fires the configurable hook. Scroll wheel always navigates sessions —
    /// the detail panel never steals focus.
    ///
    /// In TwoLine layout mode, handles clicks on session chips and overflow indicators.
    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Action {
        // Branch based on layout mode
        if self.layout_mode == LayoutMode::TwoLine {
            self.handle_mouse_event_two_line(mouse)
        } else {
            self.handle_mouse_event_large(mouse)
        }
    }

    /// Handles mouse events in Large layout mode.
    fn handle_mouse_event_large(&mut self, mouse: MouseEvent) -> Action {
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
                    // Reset history scroll when clicking a different session
                    if self.selected_index != Some(idx) {
                        self.history_scroll = 0;
                    }
                    self.selected_index = Some(idx);
                    if is_double_click {
                        self.last_click = None;
                        self.execute_hook(idx);
                        return Action::None;
                    }
                    // Single click: just focus the session (detail panel updates automatically)
                } else {
                    // Click on header or outside list → clear selection (defocus)
                    self.selected_index = None;
                }

                self.last_click = Some((now, mouse.row, mouse.column));
                Action::None
            }
            MouseEventKind::ScrollDown => {
                // Scroll always navigates sessions (detail panel never steals focus)
                self.select_next();
                Action::None
            }
            MouseEventKind::ScrollUp => {
                // Scroll always navigates sessions (detail panel never steals focus)
                self.select_previous();
                Action::None
            }
            _ => Action::None,
        }
    }

    /// Handles mouse events in TwoLine layout mode.
    ///
    /// Supports:
    /// - Click on chip → select that chip
    /// - Click on left overflow indicator → scroll left, focus new leftmost chip
    /// - Click on right overflow indicator → scroll right, focus new rightmost chip
    /// - Scroll wheel → scroll viewport left/right without changing selection
    fn handle_mouse_event_two_line(&mut self, mouse: MouseEvent) -> Action {
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

                // Only handle clicks on row 0 (session chips row)
                if mouse.row == 0 {
                    match self.calculate_clicked_chip(mouse.column) {
                        ClickTarget::Chip(idx) => {
                            // Reset history scroll when clicking a different session
                            if self.selected_index != Some(idx) {
                                self.history_scroll = 0;
                            }
                            self.selected_index = Some(idx);
                            if is_double_click {
                                self.last_click = None;
                                self.execute_hook(idx);
                                return Action::None;
                            }
                        }
                        ClickTarget::LeftOverflow => {
                            // Scroll left by 1, focus the new leftmost chip
                            self.scroll_compact_left();
                            self.selected_index = Some(self.compact_scroll_offset);
                            self.history_scroll = 0;
                        }
                        ClickTarget::RightOverflow => {
                            // Scroll right by 1, focus the new rightmost chip
                            self.scroll_compact_right();
                            // New rightmost chip is at offset + max_visible - 1
                            let max_visible = self.calculate_max_visible_chips_for_click();
                            let rightmost = (self.compact_scroll_offset + max_visible - 1)
                                .min(self.sessions.len().saturating_sub(1));
                            self.selected_index = Some(rightmost);
                            self.history_scroll = 0;
                        }
                        ClickTarget::None => {
                            // Click outside any interactive element → clear selection
                            self.selected_index = None;
                        }
                    }
                }

                self.last_click = Some((now, mouse.row, mouse.column));
                Action::None
            }
            MouseEventKind::ScrollDown => {
                // Scroll viewport right (without changing selection)
                self.scroll_compact_right();
                Action::None
            }
            MouseEventKind::ScrollUp => {
                // Scroll viewport left (without changing selection)
                self.scroll_compact_left();
                Action::None
            }
            _ => Action::None,
        }
    }

    /// Calculates max visible chips for the current terminal width.
    ///
    /// Used by click detection to determine chip boundaries.
    fn calculate_max_visible_chips_for_click(&self) -> usize {
        use crate::tui::ui::CHIP_WIDTH;
        use crate::tui::ui::OVERFLOW_INDICATOR_WIDTH;

        let width = self.terminal_width as usize;
        let content_width = width.saturating_sub(OVERFLOW_INDICATOR_WIDTH * 2);
        (content_width / CHIP_WIDTH).max(1)
    }

    /// Calculates which chip or overflow indicator was clicked.
    ///
    /// Returns the target based on column position in the session chips row.
    fn calculate_clicked_chip(&self, column: u16) -> ClickTarget {
        use crate::tui::ui::CHIP_WIDTH;
        use crate::tui::ui::OVERFLOW_INDICATOR_WIDTH;

        if self.sessions.is_empty() {
            return ClickTarget::None;
        }

        let col = column as usize;
        let overflow_left = self.compact_scroll_offset;
        let max_visible = self.calculate_max_visible_chips_for_click();
        let overflow_right = self
            .sessions
            .len()
            .saturating_sub(self.compact_scroll_offset + max_visible);

        // Check left overflow indicator area
        if overflow_left > 0 && col < OVERFLOW_INDICATOR_WIDTH {
            return ClickTarget::LeftOverflow;
        }

        // Calculate content start (after left indicator or padding)
        let content_start = OVERFLOW_INDICATOR_WIDTH;
        if col < content_start {
            return ClickTarget::None;
        }

        // Calculate chip index from column offset
        let relative_col = col - content_start;
        let chip_index = relative_col / CHIP_WIDTH;
        let global_index = self.compact_scroll_offset + chip_index;

        // Check if this is a valid chip
        let visible_end = (self.compact_scroll_offset + max_visible).min(self.sessions.len());
        if global_index < visible_end {
            return ClickTarget::Chip(global_index);
        }

        // Check right overflow indicator area
        let content_end = content_start + (max_visible * CHIP_WIDTH);
        if overflow_right > 0 && col >= content_end && col < content_end + OVERFLOW_INDICATOR_WIDTH
        {
            return ClickTarget::RightOverflow;
        }

        ClickTarget::None
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
                        Action::OpenDetail(_) => {
                            // OpenDetail action is deprecated (detail is always visible)
                            // No-op for backward compatibility
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
                            // Back action now just clears selection (defocus)
                            self.selected_index = None;
                        }
                        Action::ScrollHistoryDown => {
                            self.scroll_history_down();
                        }
                        Action::ScrollHistoryUp => {
                            self.scroll_history_up();
                        }
                        Action::CopySessionId(session_id) => match arboard::Clipboard::new() {
                            Ok(mut clipboard) => match clipboard.set_text(&session_id) {
                                Ok(()) => {
                                    tracing::debug!(
                                        "copied session ID to clipboard: {}",
                                        session_id
                                    );
                                    self.status_message = Some((
                                        "Copied session ID".to_string(),
                                        Instant::now() + Duration::from_secs(2),
                                    ));
                                }
                                Err(e) => {
                                    tracing::warn!("failed to copy to clipboard: {}", e);
                                    self.status_message = Some((
                                        format!("Copy failed: {}", e),
                                        Instant::now() + Duration::from_secs(2),
                                    ));
                                }
                            },
                            Err(e) => {
                                tracing::warn!("failed to initialize clipboard: {}", e);
                                self.status_message = Some((
                                    format!("Clipboard init failed: {}", e),
                                    Instant::now() + Duration::from_secs(2),
                                ));
                            }
                        },
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
