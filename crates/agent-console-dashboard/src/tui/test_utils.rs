//! Shared test utilities for TUI testing with ratatui TestBackend.
//!
//! Provides helper functions for creating test terminals, extracting buffer
//! content, asserting colors, and creating test session fixtures.

#![cfg(test)]

use crate::{AgentType, Session, Status, INACTIVE_SESSION_THRESHOLD};
use ratatui::{backend::TestBackend, buffer::Buffer, style::Color, Terminal};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Creates a Terminal with TestBackend at the specified dimensions.
pub fn test_terminal(width: u16, height: u16) -> Terminal<TestBackend> {
    let backend = TestBackend::new(width, height);
    Terminal::new(backend).expect("failed to create test terminal")
}

/// Extracts all text from a specific row in the buffer as a single String.
pub fn row_text(buffer: &Buffer, row: u16) -> String {
    let area = buffer.area();
    if row >= area.height {
        return String::new();
    }
    (0..area.width)
        .map(|col| {
            buffer
                .cell((col, row))
                .map(|cell| cell.symbol())
                .unwrap_or(" ")
        })
        .collect()
}

/// Checks if a specific row contains the given substring.
pub fn row_contains(buffer: &Buffer, row: u16, text: &str) -> bool {
    row_text(buffer, row).contains(text)
}

/// Finds the first row index that contains the given text, or None if not found.
pub fn find_row_with_text(buffer: &Buffer, text: &str) -> Option<u16> {
    let area = buffer.area();
    for row in 0..area.height {
        if row_contains(buffer, row, text) {
            return Some(row);
        }
    }
    None
}

/// Asserts that the cell at (col, row) has the specified foreground color.
pub fn assert_fg_color(buffer: &Buffer, col: u16, row: u16, color: Color) {
    let cell = buffer
        .cell((col, row))
        .unwrap_or_else(|| panic!("cell at ({}, {}) does not exist", col, row));
    assert_eq!(
        cell.fg, color,
        "expected fg color {:?} at ({}, {}), got {:?}",
        color, col, row, cell.fg
    );
}

/// Asserts that the cell at (col, row) has the specified background color.
pub fn assert_bg_color(buffer: &Buffer, col: u16, row: u16, color: Color) {
    let cell = buffer
        .cell((col, row))
        .unwrap_or_else(|| panic!("cell at ({}, {}) does not exist", col, row));
    assert_eq!(
        cell.bg, color,
        "expected bg color {:?} at ({}, {}), got {:?}",
        color, col, row, cell.bg
    );
}

/// Finds the first occurrence of `text` in the specified row and checks
/// if the first character of that text has the specified foreground color.
pub fn assert_text_fg_in_row(buffer: &Buffer, row: u16, text: &str, color: Color) {
    let row_string = row_text(buffer, row);
    let col = row_string
        .find(text)
        .unwrap_or_else(|| panic!("text '{}' not found in row {}: '{}'", text, row, row_string));
    assert_fg_color(buffer, col as u16, row, color);
}

/// Finds the first occurrence of `text` in the specified row and checks
/// if the first character of that text has the specified background color.
pub fn assert_text_bg_in_row(buffer: &Buffer, row: u16, text: &str, color: Color) {
    let row_string = row_text(buffer, row);
    let col = row_string
        .find(text)
        .unwrap_or_else(|| panic!("text '{}' not found in row {}: '{}'", text, row, row_string));
    assert_bg_color(buffer, col as u16, row, color);
}

/// Creates a Session with the given id, status, and working directory.
pub fn make_session(id: &str, status: Status, working_dir: Option<PathBuf>) -> Session {
    let mut session = Session::new(id.to_string(), AgentType::ClaudeCode, working_dir);
    session.status = status;
    session
}

/// Creates a Session that will be considered inactive based on the threshold.
///
/// Sets `last_activity` to `age_secs` seconds in the past, making the session
/// appear inactive when checked with `is_inactive(INACTIVE_SESSION_THRESHOLD)`.
pub fn make_inactive_session(id: &str, age_secs: u64) -> Session {
    let mut session = Session::new(
        id.to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp/test")),
    );
    // Set last_activity in the past
    let age = Duration::from_secs(age_secs);
    session.last_activity = Instant::now().checked_sub(age).unwrap_or_else(Instant::now);
    session
}

/// Renders the session list to a buffer and returns the buffer for inspection.
///
/// Creates a terminal of the specified size, renders the session list using
/// `render_session_list`, and returns the resulting buffer.
pub fn render_session_list_to_buffer(
    sessions: &[Session],
    selected: Option<usize>,
    width: u16,
    height: u16,
) -> Buffer {
    let mut terminal = test_terminal(width, height);
    terminal
        .draw(|frame| {
            let area = frame.area();
            crate::tui::views::dashboard::render_session_list(
                frame, area, sessions, selected, width,
            );
        })
        .expect("draw failed");
    terminal.backend().buffer().clone()
}

/// Renders the full dashboard to a buffer and returns the buffer for inspection.
///
/// Creates a terminal of the specified size, renders the full dashboard using
/// `render_dashboard`, and returns the resulting buffer.
pub fn render_dashboard_to_buffer(
    app: &mut crate::tui::app::App,
    width: u16,
    height: u16,
) -> Buffer {
    let mut terminal = test_terminal(width, height);
    terminal
        .draw(|frame| {
            crate::tui::ui::render_dashboard(frame, app);
        })
        .expect("draw failed");
    terminal.backend().buffer().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_terminal_creates_terminal() {
        let terminal = test_terminal(80, 24);
        let size = terminal.size().expect("should have size");
        assert_eq!(size.width, 80);
        assert_eq!(size.height, 24);
    }

    #[test]
    fn test_row_text_extracts_row_content() {
        let mut terminal = test_terminal(20, 5);
        terminal
            .draw(|frame| {
                let area = frame.area();
                let para = ratatui::widgets::Paragraph::new("Hello World");
                frame.render_widget(para, area);
            })
            .expect("draw failed");
        let buffer = terminal.backend().buffer();
        let text = row_text(buffer, 0);
        assert!(text.contains("Hello World"));
    }

    #[test]
    fn test_row_contains_finds_substring() {
        let mut terminal = test_terminal(30, 5);
        terminal
            .draw(|frame| {
                let area = frame.area();
                let para = ratatui::widgets::Paragraph::new("Test Content Here");
                frame.render_widget(para, area);
            })
            .expect("draw failed");
        let buffer = terminal.backend().buffer();
        assert!(row_contains(buffer, 0, "Content"));
        assert!(!row_contains(buffer, 0, "Missing"));
    }

    #[test]
    fn test_find_row_with_text_returns_row_index() {
        let mut terminal = test_terminal(40, 10);
        terminal
            .draw(|frame| {
                let area = frame.area();
                use ratatui::layout::{Constraint, Direction, Layout};
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(1), Constraint::Length(1)])
                    .split(area);
                frame.render_widget(ratatui::widgets::Paragraph::new("First Line"), chunks[0]);
                frame.render_widget(ratatui::widgets::Paragraph::new("Target Row"), chunks[1]);
            })
            .expect("draw failed");
        let buffer = terminal.backend().buffer();
        let row = find_row_with_text(buffer, "Target Row");
        assert_eq!(row, Some(1));
    }

    #[test]
    fn test_make_session_creates_session_with_params() {
        let session = make_session(
            "test-id",
            Status::Attention,
            Some(PathBuf::from("/home/user")),
        );
        assert_eq!(session.session_id, "test-id");
        assert_eq!(session.status, Status::Attention);
        assert_eq!(session.working_dir, Some(PathBuf::from("/home/user")));
    }

    #[test]
    fn test_make_session_with_none_working_dir() {
        let session = make_session("no-dir", Status::Working, None);
        assert_eq!(session.session_id, "no-dir");
        assert_eq!(session.working_dir, None);
    }

    #[test]
    fn test_make_inactive_session_is_inactive() {
        // Create a session that's older than the threshold
        let session =
            make_inactive_session("old-session", INACTIVE_SESSION_THRESHOLD.as_secs() + 100);
        assert!(
            session.is_inactive(INACTIVE_SESSION_THRESHOLD),
            "session should be inactive"
        );
    }

    #[test]
    fn test_make_inactive_session_with_recent_age_not_inactive() {
        // Create a session that's not old enough to be inactive
        let session = make_inactive_session("recent-session", 60);
        assert!(
            !session.is_inactive(INACTIVE_SESSION_THRESHOLD),
            "session should not be inactive"
        );
    }
}
