//! Event handling for the TUI.
//!
//! Wraps crossterm events and adds a tick variant for periodic UI refresh.

use crate::tui::app::App;
use crossterm::event::{
    Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, KeyModifiers, MouseEvent,
};
use futures::StreamExt;
use std::time::Duration;
use tokio::time::interval;

/// Application-level event variants.
#[derive(Debug, Clone, Copy)]
pub enum Event {
    /// A key was pressed.
    Key(KeyEvent),
    /// A mouse event occurred.
    Mouse(MouseEvent),
    /// Terminal was resized.
    Resize(u16, u16),
    /// Periodic tick for UI refresh.
    Tick,
}

/// Event handler that merges terminal input events with periodic ticks.
pub struct EventHandler {
    /// Tick interval duration.
    tick_rate: Duration,
}

impl EventHandler {
    /// Creates a new EventHandler with the specified tick rate.
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    /// Waits for the next event, returning either a terminal event or a tick.
    ///
    /// Uses `tokio::select!` to race between crossterm input and the tick timer.
    pub async fn next(&self, reader: &mut EventStream) -> std::io::Result<Event> {
        let mut tick = interval(self.tick_rate);
        // Consume the first immediate tick
        tick.tick().await;

        loop {
            tokio::select! {
                maybe_event = reader.next() => {
                    match maybe_event {
                        Some(Ok(CrosstermEvent::Key(key))) => return Ok(Event::Key(key)),
                        Some(Ok(CrosstermEvent::Mouse(mouse))) => return Ok(Event::Mouse(mouse)),
                        Some(Ok(CrosstermEvent::Resize(w, h))) => return Ok(Event::Resize(w, h)),
                        Some(Err(e)) => return Err(e),
                        // Ignore focus, paste events
                        Some(Ok(_)) => continue,
                        None => return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "event stream ended",
                        )),
                    }
                }
                _ = tick.tick() => {
                    return Ok(Event::Tick);
                }
            }
        }
    }
}

/// Action produced by handling a key event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// No action to take.
    None,
    /// Quit the application.
    Quit,
    /// Open detail view for the session at the given index.
    OpenDetail(usize),
    /// Resurrect the closed session with the given ID.
    Resurrect(String),
    /// Remove the session with the given ID (pending confirmation).
    Remove(String),
    /// Switch to the layout preset with the given count (1-4).
    SwitchLayout(u8),
    /// Close overlay / go back from detail view.
    Back,
    /// Scroll history down in detail view.
    ScrollHistoryDown,
    /// Scroll history up in detail view.
    ScrollHistoryUp,
    /// Copy session ID to clipboard.
    CopySessionId(String),
}

/// Handles a key event by dispatching to the appropriate app method or action.
///
/// When the detail view is active, keys are routed to detail-specific handlers
/// (scroll, resurrect, close, escape). Otherwise, dashboard navigation applies.
pub fn handle_key_event(app: &mut App, key: KeyEvent) -> Action {
    use crate::tui::app::View;

    // Global: quit always works
    match key.code {
        KeyCode::Char('q') => return Action::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Action::Quit,
        _ => {}
    }

    // Detail view key handling
    if let View::Detail { session_index, .. } = app.view {
        return handle_detail_key(app, key, session_index);
    }

    // Dashboard view key handling
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            app.select_next();
            Action::None
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.select_previous();
            Action::None
        }
        KeyCode::Left => {
            // In TwoLine mode, focus the new leftmost chip after scrolling
            if app.layout_mode == crate::tui::app::LayoutMode::TwoLine {
                app.scroll_compact_left();
                app.selected_index = Some(app.compact_scroll_offset);
                app.history_scroll = 0;
            } else {
                app.scroll_compact_left();
            }
            Action::None
        }
        KeyCode::Right => {
            // In TwoLine mode, focus the new rightmost chip after scrolling
            if app.layout_mode == crate::tui::app::LayoutMode::TwoLine {
                app.scroll_compact_right();
                // Calculate rightmost visible chip index
                let max_visible =
                    crate::tui::ui::calculate_max_visible_chips_public(app.terminal_width);
                let rightmost = (app.compact_scroll_offset + max_visible - 1)
                    .min(app.sessions.len().saturating_sub(1));
                app.selected_index = Some(rightmost);
                app.history_scroll = 0;
            } else {
                app.scroll_compact_right();
            }
            Action::None
        }
        KeyCode::Enter => {
            // Enter on focused session fires the appropriate hook (activate or reopen)
            if let Some(idx) = app.selected_index {
                app.execute_hook(idx);
            }
            Action::None
        }
        KeyCode::Char('r') => {
            // 'r' on closed session fires reopen_hook
            if let Some(idx) = app.selected_index {
                if let Some(session) = app.sessions.get(idx) {
                    if session.status == crate::Status::Closed {
                        app.execute_hook(idx);
                    }
                }
            }
            Action::None
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            // 's' copies session ID when session is selected
            if let Some(session) = app.selected_session() {
                Action::CopySessionId(session.session_id.clone())
            } else {
                Action::None
            }
        }
        KeyCode::Char('d') => {
            if let Some(session) = app.selected_session() {
                Action::Remove(session.session_id.clone())
            } else {
                Action::None
            }
        }
        KeyCode::Char(c @ '1'..='4') => Action::SwitchLayout(c as u8 - b'0'),
        KeyCode::Esc => {
            // Esc clears selection (defocus)
            app.selected_index = None;
            Action::None
        }
        _ => Action::None,
    }
}

/// Handles key events when the detail view is active.
///
/// When a `Resurrect` action is returned, the caller should use hook-based reopen
/// to execute the resurrection.
fn handle_detail_key(app: &App, key: KeyEvent, session_index: usize) -> Action {
    match key.code {
        KeyCode::Esc => Action::Back,
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if let Some(session) = app.sessions.get(session_index) {
                if session.status == crate::Status::Closed {
                    Action::Resurrect(session.session_id.clone())
                } else {
                    Action::None
                }
            } else {
                Action::None
            }
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if let Some(session) = app.sessions.get(session_index) {
                Action::Remove(session.session_id.clone())
            } else {
                Action::None
            }
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            if let Some(session) = app.sessions.get(session_index) {
                Action::CopySessionId(session.session_id.clone())
            } else {
                Action::None
            }
        }
        KeyCode::Char('j') | KeyCode::Down => Action::ScrollHistoryDown,
        KeyCode::Char('k') | KeyCode::Up => Action::ScrollHistoryUp,
        _ => Action::None,
    }
}

/// Returns true if the key event should trigger application quit.
pub fn should_quit(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('q'))
        || (key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')))
}

#[cfg(test)]
mod tests;
