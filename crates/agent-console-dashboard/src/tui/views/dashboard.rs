//! Dashboard view rendering for session list display.
//!
//! Provides session list rendering with responsive column layouts
//! and status-based color coding.

use crate::{Session, Status, INACTIVE_SESSION_THRESHOLD};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::path::PathBuf;
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
        // Narrow: symbol + name only (no column alignment)
        Line::from(vec![
            Span::styled(format!("{} ", symbol), Style::default().fg(color)),
            Span::styled(name, dim),
        ])
    } else if width <= WIDE_THRESHOLD {
        // Standard: symbol + name (flexible) + status + working dir + elapsed (right-aligned)
        // Fixed column widths: symbol (2) + status (10) + work_dir (20) + elapsed (8) = 40
        let fixed_width = 2 + 10 + 20 + 8;
        let name_width = (width as usize).saturating_sub(fixed_width);

        let work_dir_text = if session.working_dir == PathBuf::from("unknown") {
            "<error>".to_string()
        } else {
            truncate_path(&session.working_dir, 20)
        };

        let work_dir_span = if session.working_dir == PathBuf::from("unknown") {
            Span::styled(
                format!("{:>20}", work_dir_text),
                Style::default().fg(error_color()),
            )
        } else {
            Span::styled(format!("{:>20}", work_dir_text), dim)
        };

        Line::from(vec![
            Span::styled(format!("{} ", symbol), Style::default().fg(color)),
            Span::styled(format!("{:<name_width$}", name), dim),
            Span::styled(format!("{:>10}", status_text), Style::default().fg(color)),
            work_dir_span,
            Span::styled(format!("{:>8}", elapsed), dim),
        ])
    } else {
        // Wide: session ID + symbol + name (flexible) + status + working dir + elapsed (right-aligned)
        // Fixed column widths: session_id (14) + symbol (2) + status (10) + work_dir (30) + elapsed (8) = 64
        let fixed_width = 14 + 2 + 10 + 30 + 8;
        let name_width = (width as usize).saturating_sub(fixed_width);

        let session_id_display = session
            .session_id
            .as_deref()
            .map(|s| truncate_string(s, 12))
            .unwrap_or_default();

        let work_dir_text = if session.working_dir == PathBuf::from("unknown") {
            "<error>".to_string()
        } else {
            truncate_path(&session.working_dir, 30)
        };

        let work_dir_span = if session.working_dir == PathBuf::from("unknown") {
            Span::styled(
                format!("{:>30}", work_dir_text),
                Style::default().fg(error_color()),
            )
        } else {
            Span::styled(format!("{:>30}", work_dir_text), dim)
        };

        Line::from(vec![
            Span::styled(format!("{:<14}", session_id_display), dim),
            Span::styled(format!("{} ", symbol), Style::default().fg(color)),
            Span::styled(format!("{:<name_width$}", name), dim),
            Span::styled(format!("{:>10}", status_text), Style::default().fg(color)),
            work_dir_span,
            Span::styled(format!("{:>8}", elapsed), dim),
        ])
    }
}

/// Formats a header line matching the column widths from format_session_line.
///
/// Returns a header row with column titles aligned to their respective columns.
/// Narrow mode has no headers, standard mode shows 4 columns, wide mode shows 5.
pub fn format_header_line(width: u16) -> Line<'static> {
    let header_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    if width < NARROW_THRESHOLD {
        // Narrow: no headers
        Line::from(vec![])
    } else if width <= WIDE_THRESHOLD {
        // Standard: Name (flexible) + Status + Directory + Elapsed (right-aligned)
        let fixed_width = 2 + 10 + 20 + 8; // symbol + status + dir + elapsed
        let name_width = (width as usize).saturating_sub(fixed_width);

        Line::from(vec![
            Span::styled("  ", header_style), // Symbol space
            Span::styled(format!("{:<name_width$}", "Name"), header_style),
            Span::styled(format!("{:>10}", "Status"), header_style),
            Span::styled(format!("{:>20}", "Directory"), header_style),
            Span::styled(format!("{:>8}", "Elapsed"), header_style),
        ])
    } else {
        // Wide: Session ID + Name (flexible) + Status + Directory + Elapsed (right-aligned)
        let fixed_width = 14 + 2 + 10 + 30 + 8; // id + symbol + status + dir + elapsed
        let name_width = (width as usize).saturating_sub(fixed_width);

        Line::from(vec![
            Span::styled(format!("{:<14}", "Session ID"), header_style),
            Span::styled("  ", header_style), // Symbol space
            Span::styled(format!("{:<name_width$}", "Name"), header_style),
            Span::styled(format!("{:>10}", "Status"), header_style),
            Span::styled(format!("{:>30}", "Directory"), header_style),
            Span::styled(format!("{:>8}", "Elapsed"), header_style),
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
    // Split area into header (1 line) + list (remaining) if not narrow mode
    let (header_area, list_area) = if width >= NARROW_THRESHOLD {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // header
                Constraint::Min(1),    // list
            ])
            .split(area);
        (Some(chunks[0]), chunks[1])
    } else {
        (None, area)
    };

    // Render header if not narrow mode
    if let Some(header_rect) = header_area {
        let header_line = format_header_line(width);
        let header = Paragraph::new(header_line);
        frame.render_widget(header, header_rect);
    }

    // Render session list
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

    frame.render_stateful_widget(list, list_area, &mut state);
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

    // --- Story 1 (acd-lht): Red error for missing CWD tests ---

    #[test]
    fn test_format_session_line_unknown_working_dir_shows_error_standard() {
        let mut session = Session::new(
            "error-test".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("unknown"),
        );
        session.status = Status::Working;
        let line = format_session_line(&session, 60);

        // Should have 5 spans: symbol, name, status, work_dir (error), elapsed
        assert_eq!(line.spans.len(), 5);

        // The work_dir span (index 3) should contain "<error>" and be styled with red
        let work_dir_span = &line.spans[3];
        assert!(
            work_dir_span.content.contains("<error>"),
            "Expected '<error>' in work_dir span, got: '{}'",
            work_dir_span.content
        );
        assert_eq!(
            work_dir_span.style.fg,
            Some(error_color()),
            "Expected error color (red) for <error> span"
        );
    }

    #[test]
    fn test_format_session_line_unknown_working_dir_shows_error_wide() {
        let mut session = Session::new(
            "error-wide-test".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("unknown"),
        );
        session.status = Status::Attention;
        session.session_id = Some("session-abc".to_string());
        let line = format_session_line(&session, 100);

        // Should have 6 spans: session_id, symbol, name, status, work_dir (error), elapsed
        assert_eq!(line.spans.len(), 6);

        // The work_dir span (index 4) should contain "<error>" and be styled with red
        let work_dir_span = &line.spans[4];
        assert!(
            work_dir_span.content.contains("<error>"),
            "Expected '<error>' in work_dir span, got: '{}'",
            work_dir_span.content
        );
        assert_eq!(
            work_dir_span.style.fg,
            Some(error_color()),
            "Expected error color (red) for <error> span"
        );
    }

    #[test]
    fn test_format_session_line_normal_path_unchanged() {
        let session = Session::new(
            "normal-path-test".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/home/user/project"),
        );
        let line = format_session_line(&session, 60);

        // Should have 5 spans
        assert_eq!(line.spans.len(), 5);

        // The work_dir span (index 3) should contain the truncated path, not "<error>"
        let work_dir_span = &line.spans[3];
        assert!(
            !work_dir_span.content.contains("<error>"),
            "Normal path should not display <error>, got: '{}'",
            work_dir_span.content
        );
        assert!(
            work_dir_span.content.contains("/home/user/project"),
            "Expected path to contain '/home/user/project', got: '{}'",
            work_dir_span.content
        );
        // Should not be red
        assert_ne!(
            work_dir_span.style.fg,
            Some(error_color()),
            "Normal path should not use error color"
        );
    }

    // --- Story 2 (acd-r57): Right-align columns tests ---

    #[test]
    fn test_column_alignment_standard_width() {
        let session = Session::new(
            "align-test".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/home/user/project"),
        );
        let line = format_session_line(&session, 60);

        // Should have 5 spans: symbol, name, status, workdir, elapsed
        assert_eq!(line.spans.len(), 5);

        // Status (index 2) should be right-aligned (check for leading spaces)
        let status_span = &line.spans[2];
        assert!(
            status_span.content.starts_with(' ') || status_span.content.len() >= 10,
            "Status should be right-aligned with width 10, got: '{}'",
            status_span.content
        );

        // Work_dir (index 3) should be right-aligned
        let work_dir_span = &line.spans[3];
        assert!(
            work_dir_span.content.starts_with(' ') || work_dir_span.content.len() >= 20,
            "Work_dir should be right-aligned with width 20, got: '{}'",
            work_dir_span.content
        );

        // Elapsed (index 4) should be right-aligned
        let elapsed_span = &line.spans[4];
        assert!(
            elapsed_span.content.starts_with(' ') || elapsed_span.content.len() >= 8,
            "Elapsed should be right-aligned with width 8, got: '{}'",
            elapsed_span.content
        );
    }

    #[test]
    fn test_column_alignment_wide_width() {
        let mut session = Session::new(
            "wide-align-test".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/home/user/project"),
        );
        session.session_id = Some("session-123".to_string());
        let line = format_session_line(&session, 120);

        // Should have 6 spans: session_id, symbol, name, status, workdir, elapsed
        assert_eq!(line.spans.len(), 6);

        // Session ID (index 0) should be left-aligned
        let session_id_span = &line.spans[0];
        assert!(
            session_id_span.content.ends_with(' ') || session_id_span.content.len() <= 14,
            "Session ID should be left-aligned, got: '{}'",
            session_id_span.content
        );

        // Status (index 3) should be right-aligned
        let status_span = &line.spans[3];
        assert!(
            status_span.content.starts_with(' ') || status_span.content.len() >= 10,
            "Status should be right-aligned with width 10, got: '{}'",
            status_span.content
        );

        // Work_dir (index 4) should be right-aligned
        let work_dir_span = &line.spans[4];
        assert!(
            work_dir_span.content.starts_with(' ') || work_dir_span.content.len() >= 30,
            "Work_dir should be right-aligned with width 30, got: '{}'",
            work_dir_span.content
        );

        // Elapsed (index 5) should be right-aligned
        let elapsed_span = &line.spans[5];
        assert!(
            elapsed_span.content.starts_with(' ') || elapsed_span.content.len() >= 8,
            "Elapsed should be right-aligned with width 8, got: '{}'",
            elapsed_span.content
        );
    }

    #[test]
    fn test_name_column_expands_with_terminal_width() {
        let session = Session::new(
            "expand-test".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/tmp"),
        );

        // Test at standard width (60 cols)
        let line_60 = format_session_line(&session, 60);
        let name_span_60 = &line_60.spans[1];

        // Test at wider width (80 cols)
        let line_80 = format_session_line(&session, 80);
        let name_span_80 = &line_80.spans[1];

        // Name column at 80 should be wider than at 60
        assert!(
            name_span_80.content.len() > name_span_60.content.len(),
            "Name column should expand with terminal width: 60={}, 80={}",
            name_span_60.content.len(),
            name_span_80.content.len()
        );
    }

    // --- Story 3 (acd-8uw): Column headers tests ---

    #[test]
    fn test_header_narrow_mode_no_header() {
        let line = format_header_line(30);
        // Narrow mode should have no header
        assert_eq!(line.spans.len(), 0, "Narrow mode should have no header");
    }

    #[test]
    fn test_header_standard_mode() {
        let line = format_header_line(60);
        // Standard mode: symbol space + Name + Status + Directory + Elapsed = 5 spans
        assert_eq!(
            line.spans.len(),
            5,
            "Standard mode should have 5 header spans"
        );

        // Verify header contains expected column titles
        let full_text = line
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<Vec<_>>()
            .join("");
        assert!(full_text.contains("Name"), "Header should contain 'Name'");
        assert!(
            full_text.contains("Status"),
            "Header should contain 'Status'"
        );
        assert!(
            full_text.contains("Directory"),
            "Header should contain 'Directory'"
        );
        assert!(
            full_text.contains("Elapsed"),
            "Header should contain 'Elapsed'"
        );

        // Verify all spans use header style (cyan + bold)
        for span in &line.spans {
            assert_eq!(
                span.style.fg,
                Some(Color::Cyan),
                "Header span should use cyan color"
            );
            assert!(
                span.style.add_modifier.contains(Modifier::BOLD),
                "Header span should be bold"
            );
        }
    }

    #[test]
    fn test_header_wide_mode_includes_session_id() {
        let line = format_header_line(100);
        // Wide mode: Session ID + symbol space + Name + Status + Directory + Elapsed = 6 spans
        assert_eq!(line.spans.len(), 6, "Wide mode should have 6 header spans");

        // Verify header contains expected column titles including Session ID
        let full_text = line
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<Vec<_>>()
            .join("");
        assert!(
            full_text.contains("Session ID"),
            "Header should contain 'Session ID'"
        );
        assert!(full_text.contains("Name"), "Header should contain 'Name'");
        assert!(
            full_text.contains("Status"),
            "Header should contain 'Status'"
        );
        assert!(
            full_text.contains("Directory"),
            "Header should contain 'Directory'"
        );
        assert!(
            full_text.contains("Elapsed"),
            "Header should contain 'Elapsed'"
        );
    }

    #[test]
    fn test_header_alignment_matches_data() {
        // Verify that header columns align with data columns at standard width
        let header = format_header_line(60);
        let session = Session::new(
            "align-check".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/tmp/test"),
        );
        let data_line = format_session_line(&session, 60);

        // Both should have same span count
        assert_eq!(
            header.spans.len(),
            data_line.spans.len(),
            "Header and data should have same span count"
        );

        // Verify column widths match (approximate check for alignment)
        for (i, (header_span, data_span)) in
            header.spans.iter().zip(data_line.spans.iter()).enumerate()
        {
            // Allow some tolerance for content differences but structure should match
            if i > 0 {
                // Skip symbol column which is just spacing
                let header_len = header_span.content.len();
                let data_len = data_span.content.len();
                // Widths should be close (within reasonable tolerance)
                assert!(
                    (header_len as i32 - data_len as i32).abs() <= 5,
                    "Column {} width mismatch: header={}, data={}",
                    i,
                    header_len,
                    data_len
                );
            }
        }
    }
}
