//! Dashboard view rendering for session list display.
//!
//! Provides session list rendering with responsive column layouts
//! and status-based color coding.

use crate::{Session, Status, INACTIVE_SESSION_THRESHOLD};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use std::time::Instant;

/// Returns the status symbol for a given session status.
pub fn status_symbol(status: Status) -> &'static str {
    match status {
        Status::Working => "●",
        Status::Attention => "○",
        Status::Question => "?",
        Status::Closed => "×",
    }
}

/// Returns the display color for a given session status.
pub fn status_color(status: Status) -> Color {
    match status {
        Status::Working => Color::Green,
        Status::Attention => Color::Yellow,
        Status::Question => Color::Blue,
        Status::Closed => Color::Gray,
    }
}

/// Returns the color for an error status (used for sessions with errors).
pub fn error_color() -> Color {
    Color::Red
}

/// Formats the elapsed time since the given instant as a human-readable string.
///
/// Returns "Xh Ym" for durations >= 1 hour, "Xm Ys" otherwise, or "Xs" for < 1 minute.
pub fn format_elapsed(since: Instant) -> String {
    let elapsed = since.elapsed();
    let total_seconds = elapsed.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Formats the elapsed time from a raw seconds value (for testing without Instant).
pub fn format_elapsed_seconds(total_seconds: u64) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Responsive layout breakpoint thresholds.
const NARROW_THRESHOLD: u16 = 40;
const WIDE_THRESHOLD: u16 = 80;

/// Formats a single session line based on available terminal width.
///
/// Responsive breakpoints:
/// - `<40` cols: symbol + name only
/// - `40-80` cols: symbol + name + status + working dir + elapsed
/// - `>80` cols: full with session ID prefix
pub fn format_session_line<'a>(session: &Session, width: u16) -> Line<'a> {
    let inactive = session.is_inactive(INACTIVE_SESSION_THRESHOLD);
    let (color, symbol, dim, status_text) = if inactive {
        (
            Color::DarkGray,
            "◌",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
            "inactive".to_string(),
        )
    } else {
        (
            status_color(session.status),
            status_symbol(session.status),
            Style::default(),
            session.status.to_string(),
        )
    };
    let elapsed = format_elapsed(session.since);
    let name = truncate_string(&session.id, 20);

    if width < NARROW_THRESHOLD {
        // Narrow: symbol + name only
        Line::from(vec![
            Span::styled(format!("{} ", symbol), Style::default().fg(color)),
            Span::styled(name, dim),
        ])
    } else if width <= WIDE_THRESHOLD {
        // Standard: symbol + name + status + working dir + elapsed
        let work_dir = truncate_path(&session.working_dir, 20);
        Line::from(vec![
            Span::styled(format!("{} ", symbol), Style::default().fg(color)),
            Span::styled(format!("{:<20} ", name), dim),
            Span::styled(format!("{:<10} ", status_text), Style::default().fg(color)),
            Span::styled(format!("{:<20} ", work_dir), dim),
            Span::styled(elapsed, dim),
        ])
    } else {
        // Wide: session ID prefix + symbol + name + status + working dir + elapsed
        let short_id = truncate_string(&session.id, 8);
        let work_dir = truncate_path(&session.working_dir, 30);
        let session_id_display = session
            .session_id
            .as_deref()
            .map(|s| truncate_string(s, 12))
            .unwrap_or_default();

        Line::from(vec![
            Span::styled(format!("{:<14} ", session_id_display), dim),
            Span::styled(format!("{} ", symbol), Style::default().fg(color)),
            Span::styled(format!("{:<20} ", short_id), dim),
            Span::styled(format!("{:<10} ", status_text), Style::default().fg(color)),
            Span::styled(format!("{:<30} ", work_dir), dim),
            Span::styled(elapsed, dim),
        ])
    }
}

/// Renders the session list into the given area.
pub fn render_session_list(
    frame: &mut Frame,
    area: Rect,
    sessions: &[Session],
    selected_index: Option<usize>,
    width: u16,
) {
    let items: Vec<ListItem> = sessions
        .iter()
        .map(|session| ListItem::new(format_session_line(session, width)))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::TOP | Borders::BOTTOM)
                .title(" Sessions "),
        )
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("▸ ");

    let mut state = ListState::default();
    state.select(selected_index);

    frame.render_stateful_widget(list, area, &mut state);
}

/// Truncates a string to the given max length, appending "..." if truncated.
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        s[..max_len].to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Truncates a path display to the given max length.
fn truncate_path(path: &std::path::Path, max_len: usize) -> String {
    let display = path.display().to_string();
    truncate_string(&display, max_len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentType, Session};
    use std::path::PathBuf;

    fn make_session(id: &str, status: Status) -> Session {
        let mut s = Session::new(
            id.to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/home/user/project"),
        );
        s.status = status;
        s
    }

    // --- status_symbol tests ---

    #[test]
    fn test_status_symbol_working() {
        assert_eq!(status_symbol(Status::Working), "●");
    }

    #[test]
    fn test_status_symbol_attention() {
        assert_eq!(status_symbol(Status::Attention), "○");
    }

    #[test]
    fn test_status_symbol_question() {
        assert_eq!(status_symbol(Status::Question), "?");
    }

    #[test]
    fn test_status_symbol_closed() {
        assert_eq!(status_symbol(Status::Closed), "×");
    }

    // --- status_color tests ---

    #[test]
    fn test_status_color_working() {
        assert_eq!(status_color(Status::Working), Color::Green);
    }

    #[test]
    fn test_status_color_attention() {
        assert_eq!(status_color(Status::Attention), Color::Yellow);
    }

    #[test]
    fn test_status_color_question() {
        assert_eq!(status_color(Status::Question), Color::Blue);
    }

    #[test]
    fn test_status_color_closed() {
        assert_eq!(status_color(Status::Closed), Color::Gray);
    }

    #[test]
    fn test_error_color() {
        assert_eq!(error_color(), Color::Red);
    }

    // --- format_elapsed_seconds tests ---

    #[test]
    fn test_format_elapsed_seconds_zero() {
        assert_eq!(format_elapsed_seconds(0), "0s");
    }

    #[test]
    fn test_format_elapsed_seconds_under_minute() {
        assert_eq!(format_elapsed_seconds(45), "45s");
    }

    #[test]
    fn test_format_elapsed_seconds_minutes() {
        assert_eq!(format_elapsed_seconds(125), "2m 5s");
    }

    #[test]
    fn test_format_elapsed_seconds_hours() {
        assert_eq!(format_elapsed_seconds(3661), "1h 1m");
    }

    #[test]
    fn test_format_elapsed_seconds_exact_hour() {
        assert_eq!(format_elapsed_seconds(3600), "1h 0m");
    }

    #[test]
    fn test_format_elapsed_seconds_exact_minute() {
        assert_eq!(format_elapsed_seconds(60), "1m 0s");
    }

    // --- truncate_string tests ---

    #[test]
    fn test_truncate_string_short() {
        assert_eq!(truncate_string("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_string_exact() {
        assert_eq!(truncate_string("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_string_long() {
        assert_eq!(truncate_string("hello world!", 8), "hello...");
    }

    #[test]
    fn test_truncate_string_very_short_max() {
        assert_eq!(truncate_string("hello", 2), "he");
    }

    #[test]
    fn test_truncate_string_empty() {
        assert_eq!(truncate_string("", 10), "");
    }

    // --- truncate_path tests ---

    #[test]
    fn test_truncate_path_short() {
        let path = PathBuf::from("/tmp");
        assert_eq!(truncate_path(&path, 20), "/tmp");
    }

    #[test]
    fn test_truncate_path_long() {
        let path = PathBuf::from("/very/long/path/to/some/directory");
        let result = truncate_path(&path, 15);
        assert_eq!(result.len(), 15);
        assert!(result.ends_with("..."));
    }

    // --- format_session_line tests ---

    #[test]
    fn test_format_session_line_narrow() {
        let session = make_session("my-session", Status::Working);
        let line = format_session_line(&session, 30);
        // Should have exactly 2 spans (symbol + name)
        assert_eq!(line.spans.len(), 2);
    }

    #[test]
    fn test_format_session_line_standard() {
        let session = make_session("my-session", Status::Attention);
        let line = format_session_line(&session, 60);
        // Should have 5 spans (symbol, name, status, workdir, elapsed)
        assert_eq!(line.spans.len(), 5);
    }

    #[test]
    fn test_format_session_line_wide() {
        let mut session = make_session("my-session", Status::Question);
        session.session_id = Some("claude-abc123".to_string());
        let line = format_session_line(&session, 100);
        // Should have 6 spans (session_id, symbol, name, status, workdir, elapsed)
        assert_eq!(line.spans.len(), 6);
    }

    #[test]
    fn test_format_session_line_wide_no_session_id() {
        let session = make_session("my-session", Status::Working);
        let line = format_session_line(&session, 100);
        // Should still have 6 spans, session_id span is empty
        assert_eq!(line.spans.len(), 6);
    }

    #[test]
    fn test_format_session_line_all_statuses() {
        for status in [
            Status::Working,
            Status::Attention,
            Status::Question,
            Status::Closed,
        ] {
            let session = make_session("test", status);
            // Should not panic at any width
            let _ = format_session_line(&session, 20);
            let _ = format_session_line(&session, 50);
            let _ = format_session_line(&session, 120);
        }
    }

    // --- render_session_list tests (no-panic) ---

    #[test]
    fn test_render_session_list_empty_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_session_list(frame, area, &[], None, 80);
            })
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_session_list_single_session_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let sessions = vec![make_session("session-1", Status::Working)];
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_session_list(frame, area, &sessions, Some(0), 80);
            })
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_session_list_many_sessions_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let sessions: Vec<Session> = (0..50)
            .map(|i| make_session(&format!("session-{}", i), Status::Working))
            .collect();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_session_list(frame, area, &sessions, Some(25), 80);
            })
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_session_list_narrow_terminal_no_panic() {
        let backend = ratatui::backend::TestBackend::new(20, 10);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let sessions = vec![make_session("narrow-test", Status::Attention)];
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_session_list(frame, area, &sessions, Some(0), 20);
            })
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_session_list_wide_terminal_no_panic() {
        let backend = ratatui::backend::TestBackend::new(200, 50);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let sessions = vec![
            make_session("wide-1", Status::Working),
            make_session("wide-2", Status::Question),
        ];
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_session_list(frame, area, &sessions, None, 200);
            })
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_session_list_selected_out_of_bounds_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let sessions = vec![make_session("only-one", Status::Closed)];
        terminal
            .draw(|frame| {
                let area = frame.area();
                // selected_index beyond session count
                render_session_list(frame, area, &sessions, Some(99), 80);
            })
            .expect("draw should not fail");
    }
}
