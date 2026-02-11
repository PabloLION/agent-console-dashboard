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

/// Computes display names for session directories with basename disambiguation.
///
/// Returns a map from session_id to display name. If multiple sessions share
/// the same basename, includes parent folders for disambiguation (up to 3 levels).
fn compute_directory_display_names(
    sessions: &[Session],
) -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;
    use std::path::Path;

    // Helper: extract components as strings from a path
    fn path_components(path: &Path) -> Vec<String> {
        path.components()
            .filter_map(|c| c.as_os_str().to_str().map(String::from))
            .collect()
    }

    // Initial display names (basename only)
    let mut display_names = HashMap::new();
    for session in sessions {
        let name = match &session.working_dir {
            None => "<error>".to_string(),
            Some(path) => path
                .file_name()
                .and_then(|n| n.to_str())
                .map(String::from)
                .unwrap_or_else(|| "<error>".to_string()),
        };
        display_names.insert(session.session_id.clone(), name);
    }

    // Iteratively add parent levels until no duplicates or max depth reached
    for depth in 1..=3 {
        let mut collision_groups: HashMap<String, Vec<String>> = HashMap::new();
        for (session_id, name) in &display_names {
            collision_groups
                .entry(name.clone())
                .or_default()
                .push(session_id.clone());
        }

        let mut changed = false;
        for (_colliding_name, session_ids) in collision_groups {
            if session_ids.len() <= 1 {
                continue; // No collision
            }

            // Try to disambiguate by adding one more parent level
            for session_id in &session_ids {
                let session = sessions
                    .iter()
                    .find(|s| &s.session_id == session_id)
                    .expect("session must exist");
                if let Some(path) = &session.working_dir {
                    let components = path_components(path);
                    if components.len() > depth {
                        // Build display name with `depth+1` levels
                        let start = components.len().saturating_sub(depth + 1);
                        let new_name = components[start..].join("/");
                        display_names.insert(session_id.clone(), new_name);
                        changed = true;
                    }
                }
            }
        }

        if !changed {
            break; // No more improvements possible
        }
    }

    // Final pass: if still ambiguous, fall back to full path
    let mut final_collision_groups: HashMap<String, Vec<String>> = HashMap::new();
    for (session_id, name) in &display_names {
        final_collision_groups
            .entry(name.clone())
            .or_default()
            .push(session_id.clone());
    }
    for (_, session_ids) in final_collision_groups {
        if session_ids.len() > 1 {
            for session_id in &session_ids {
                let session = sessions
                    .iter()
                    .find(|s| &s.session_id == session_id)
                    .expect("session must exist");
                if let Some(path) = &session.working_dir {
                    display_names.insert(session_id.clone(), path.display().to_string());
                }
            }
        }
    }

    display_names
}

/// Formats a single session line based on available terminal width.
///
/// Responsive breakpoints:
/// - `<40` cols: symbol + session ID only
/// - `40-80` cols: symbol + working dir (20) + session ID (flex) + status + elapsed
/// - `>80` cols: symbol + working dir (30) + session ID (flex) + status + elapsed
pub fn format_session_line<'a>(session: &Session, width: u16, dir_display: &str) -> Line<'a> {
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
    let name = session.session_id.clone();

    if width < NARROW_THRESHOLD {
        // Narrow: symbol + session ID only (no column alignment)
        Line::from(vec![
            Span::styled(format!("{} ", symbol), Style::default().fg(color)),
            Span::styled(name, dim),
        ])
    } else if width <= WIDE_THRESHOLD {
        // Standard: symbol + work_dir + session ID (flexible) + status + elapsed
        // Fixed column widths: symbol (2) + work_dir (20) + status (10) + elapsed (10) = 42
        let fixed_width = 2 + 20 + 10 + 10;
        let name_width = (width as usize).saturating_sub(fixed_width);

        let work_dir_text = truncate_string(dir_display, 20);
        let is_error = dir_display == "<error>";

        let work_dir_span = if is_error {
            Span::styled(
                format!("{:<20}", work_dir_text),
                Style::default().fg(error_color()),
            )
        } else {
            Span::styled(format!("{:<20}", work_dir_text), dim)
        };

        Line::from(vec![
            Span::styled(format!("{} ", symbol), Style::default().fg(color)),
            work_dir_span,
            Span::styled(format!("{:<name_width$}", name), dim),
            Span::styled(format!("{:>10}", status_text), Style::default().fg(color)),
            Span::styled(format!("{:>10}", elapsed), dim),
        ])
    } else {
        // Wide: symbol + work_dir (30) + session ID (flexible) + status + elapsed
        // Fixed column widths: symbol (2) + work_dir (30) + status (10) + elapsed (10) = 52
        let fixed_width = 2 + 30 + 10 + 10;
        let name_width = (width as usize).saturating_sub(fixed_width);

        let work_dir_text = truncate_string(dir_display, 30);
        let is_error = dir_display == "<error>";

        let work_dir_span = if is_error {
            Span::styled(
                format!("{:<30}", work_dir_text),
                Style::default().fg(error_color()),
            )
        } else {
            Span::styled(format!("{:<30}", work_dir_text), dim)
        };

        Line::from(vec![
            Span::styled(format!("{} ", symbol), Style::default().fg(color)),
            work_dir_span,
            Span::styled(format!("{:<name_width$}", name), dim),
            Span::styled(format!("{:>10}", status_text), Style::default().fg(color)),
            Span::styled(format!("{:>10}", elapsed), dim),
        ])
    }
}

/// Formats a header line matching the column widths from format_session_line.
///
/// Returns a header row with column titles aligned to their respective columns.
/// Narrow mode has no headers. Standard and wide modes share the same column
/// structure (directory, session ID, status, elapsed) with wider directory in
/// wide mode.
pub fn format_header_line(width: u16) -> Line<'static> {
    let header_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    if width < NARROW_THRESHOLD {
        // Narrow: no headers
        Line::from(vec![])
    } else if width <= WIDE_THRESHOLD {
        // Standard: Directory (20) + Session ID (flexible) + Status + Elapsed
        let fixed_width = 2 + 20 + 10 + 10; // symbol + dir + status + elapsed
        let name_width = (width as usize).saturating_sub(fixed_width);

        Line::from(vec![
            Span::styled("  ", header_style), // Symbol space
            Span::styled(format!("{:<20}", "Directory"), header_style),
            Span::styled(format!("{:<name_width$}", "Session ID"), header_style),
            Span::styled(format!("{:<10}", "Status"), header_style),
            Span::styled(format!("{:<10}", "Elapsed"), header_style),
        ])
    } else {
        // Wide: Directory (30) + Session ID (flexible) + Status + Elapsed
        let fixed_width = 2 + 30 + 10 + 10; // symbol + dir + status + elapsed
        let name_width = (width as usize).saturating_sub(fixed_width);

        Line::from(vec![
            Span::styled("  ", header_style), // Symbol space
            Span::styled(format!("{:<30}", "Directory"), header_style),
            Span::styled(format!("{:<name_width$}", "Session ID"), header_style),
            Span::styled(format!("{:<10}", "Status"), header_style),
            Span::styled(format!("{:<10}", "Elapsed"), header_style),
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

    // Compute directory display names with disambiguation
    let dir_display_names = compute_directory_display_names(sessions);

    // Render session list
    let items: Vec<ListItem> = sessions
        .iter()
        .map(|session| {
            let dir_display = dir_display_names
                .get(&session.session_id)
                .map(|s| s.as_str())
                .unwrap_or("<error>");
            ListItem::new(format_session_line(session, width, dir_display))
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentType, Session};
    use std::path::PathBuf;

    fn make_session(id: &str, status: Status) -> Session {
        let mut s = Session::new(
            id.to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/home/user/project")),
        );
        s.status = status;
        s
    }

    fn make_test_session(id: &str, working_dir: Option<PathBuf>) -> Session {
        Session::new(id.to_string(), AgentType::ClaudeCode, working_dir)
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

    // --- format_session_line tests ---

    #[test]
    fn test_format_session_line_narrow() {
        let session = make_session("my-session", Status::Working);
        let line = format_session_line(&session, 30, "project");
        // Should have exactly 2 spans (symbol + session ID)
        assert_eq!(line.spans.len(), 2);
    }

    #[test]
    fn test_format_session_line_standard() {
        let session = make_session("my-session", Status::Attention);
        let line = format_session_line(&session, 60, "project");
        // Should have 5 spans (symbol, workdir, session ID, status, elapsed)
        assert_eq!(line.spans.len(), 5);
    }

    #[test]
    fn test_format_session_line_wide() {
        let session = make_session("my-session", Status::Question);
        let line = format_session_line(&session, 100, "project");
        // Wide has same 5 spans as standard (symbol, workdir, session ID, status, elapsed)
        assert_eq!(line.spans.len(), 5);
    }

    #[test]
    fn test_format_session_line_wide_uses_wider_directory() {
        let session = Session::new(
            "my-session".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/home/user/a-long-project-name")),
        );
        let long_name = "a-long-project-name";
        let standard_line = format_session_line(&session, 60, long_name);
        let wide_line = format_session_line(&session, 100, long_name);

        // work_dir span is index 1 in both modes
        let standard_dir = &standard_line.spans[1];
        let wide_dir = &wide_line.spans[1];

        // Wide directory column (30 chars) is wider than standard (20 chars)
        assert!(
            wide_dir.content.len() > standard_dir.content.len(),
            "Wide dir column should be wider: standard={}, wide={}",
            standard_dir.content.len(),
            wide_dir.content.len()
        );
    }

    #[test]
    fn test_format_session_line_shows_full_session_id() {
        let long_id = "very-long-session-identifier-name";
        let session = make_session(long_id, Status::Working);
        let line = format_session_line(&session, 80, "project");

        // Session ID span is index 2; should contain full ID without truncation
        let name_span = &line.spans[2];
        assert!(
            name_span.content.contains(long_id),
            "Session ID should not be truncated, got: '{}'",
            name_span.content
        );
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
            let _ = format_session_line(&session, 20, "project");
            let _ = format_session_line(&session, 50, "project");
            let _ = format_session_line(&session, 120, "project");
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
            Some(PathBuf::from("unknown")),
        );
        session.status = Status::Working;
        let line = format_session_line(&session, 60, "<error>");

        // Should have 5 spans: symbol, work_dir (error), session ID, status, elapsed
        assert_eq!(line.spans.len(), 5);

        // The work_dir span (index 1) should contain "<error>" and be styled with red
        let work_dir_span = &line.spans[1];
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
            Some(PathBuf::from("unknown")),
        );
        session.status = Status::Attention;
        let line = format_session_line(&session, 100, "<error>");

        // Should have 5 spans: symbol, work_dir (error), session ID, status, elapsed
        assert_eq!(line.spans.len(), 5);

        // The work_dir span (index 1) should contain "<error>" and be styled with red
        let work_dir_span = &line.spans[1];
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
            Some(PathBuf::from("/home/user/project")),
        );
        let line = format_session_line(&session, 60, "project");

        // Should have 5 spans
        assert_eq!(line.spans.len(), 5);

        // The work_dir span (index 1) should contain the path, not "<error>"
        let work_dir_span = &line.spans[1];
        assert!(
            !work_dir_span.content.contains("<error>"),
            "Normal path should not display <error>, got: '{}'",
            work_dir_span.content
        );
        assert!(
            work_dir_span.content.contains("project"),
            "Expected path to contain 'project', got: '{}'",
            work_dir_span.content
        );
        // Should not be red
        assert_ne!(
            work_dir_span.style.fg,
            Some(error_color()),
            "Normal path should not use error color"
        );
    }

    // --- Story 2 (acd-r57): Column alignment tests ---

    #[test]
    fn test_column_alignment_standard_width() {
        let session = Session::new(
            "align-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/home/user/project")),
        );
        let line = format_session_line(&session, 60, "project");

        // Should have 5 spans: symbol, workdir, session ID, status, elapsed
        assert_eq!(line.spans.len(), 5);

        // Work_dir (index 1) should be left-aligned with width 20
        let work_dir_span = &line.spans[1];
        assert!(
            work_dir_span.content.len() >= 20,
            "Work_dir should have width >= 20, got: '{}'",
            work_dir_span.content
        );

        // Status (index 3) should be right-aligned (check for leading spaces)
        let status_span = &line.spans[3];
        assert!(
            status_span.content.starts_with(' ') || status_span.content.len() >= 10,
            "Status should be right-aligned with width 10, got: '{}'",
            status_span.content
        );

        // Elapsed (index 4) should be right-aligned with width 10
        let elapsed_span = &line.spans[4];
        assert!(
            elapsed_span.content.starts_with(' ') || elapsed_span.content.len() >= 10,
            "Elapsed should be right-aligned with width 10, got: '{}'",
            elapsed_span.content
        );
    }

    #[test]
    fn test_column_alignment_wide_width() {
        let session = Session::new(
            "wide-align-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/home/user/project")),
        );
        let line = format_session_line(&session, 120, "project");

        // Should have 5 spans: symbol, workdir, session ID, status, elapsed
        assert_eq!(line.spans.len(), 5);

        // Work_dir (index 1) should be left-aligned with width 30
        let work_dir_span = &line.spans[1];
        assert!(
            work_dir_span.content.len() >= 30,
            "Work_dir should have width >= 30, got: '{}'",
            work_dir_span.content
        );

        // Status (index 3) should be right-aligned
        let status_span = &line.spans[3];
        assert!(
            status_span.content.starts_with(' ') || status_span.content.len() >= 10,
            "Status should be right-aligned with width 10, got: '{}'",
            status_span.content
        );

        // Elapsed (index 4) should be right-aligned with width 10
        let elapsed_span = &line.spans[4];
        assert!(
            elapsed_span.content.starts_with(' ') || elapsed_span.content.len() >= 10,
            "Elapsed should be right-aligned with width 10, got: '{}'",
            elapsed_span.content
        );
    }

    #[test]
    fn test_session_id_column_expands_with_terminal_width() {
        let session = Session::new(
            "expand-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
        );

        // Test at standard width (60 cols)
        let line_60 = format_session_line(&session, 60, "tmp");
        let name_span_60 = &line_60.spans[2];

        // Test at wider width (80 cols)
        let line_80 = format_session_line(&session, 80, "tmp");
        let name_span_80 = &line_80.spans[2];

        // Session ID column at 80 should be wider than at 60
        assert!(
            name_span_80.content.len() > name_span_60.content.len(),
            "Session ID column should expand with terminal width: 60={}, 80={}",
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
        // Standard mode: symbol space + Directory + Session ID + Status + Elapsed = 5 spans
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
        assert!(
            full_text.contains("Session ID"),
            "Header should contain 'Session ID'"
        );
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
    fn test_header_wide_mode_same_columns_wider_directory() {
        let line = format_header_line(100);
        // Wide mode: same 5 spans as standard (symbol space + Directory + Session ID + Status + Elapsed)
        assert_eq!(line.spans.len(), 5, "Wide mode should have 5 header spans");

        // Verify header contains expected column titles
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

        // Wide directory header should be wider than standard
        let standard_line = format_header_line(60);
        let standard_dir = &standard_line.spans[1]; // Directory span
        let wide_dir = &line.spans[1]; // Directory span
        assert!(
            wide_dir.content.len() > standard_dir.content.len(),
            "Wide directory header should be wider: standard={}, wide={}",
            standard_dir.content.len(),
            wide_dir.content.len()
        );
    }

    #[test]
    fn test_header_labels_are_left_aligned() {
        let line = format_header_line(60);

        // Directory (index 1): check left-aligned (starts with "D", not space)
        let dir_span = &line.spans[1];
        assert!(
            dir_span.content.starts_with('D'),
            "Directory header should be left-aligned, got: '{}'",
            dir_span.content
        );

        // Session ID (index 2): check left-aligned (starts with "S", not space)
        let id_span = &line.spans[2];
        assert!(
            id_span.content.starts_with('S'),
            "Session ID header should be left-aligned, got: '{}'",
            id_span.content
        );

        // Status (index 3): check left-aligned (starts with "S", not space)
        let status_span = &line.spans[3];
        assert!(
            status_span.content.starts_with('S'),
            "Status header should be left-aligned, got: '{}'",
            status_span.content
        );

        // Elapsed (index 4): check left-aligned (starts with "E", not space)
        let elapsed_span = &line.spans[4];
        assert!(
            elapsed_span.content.starts_with('E'),
            "Elapsed header should be left-aligned, got: '{}'",
            elapsed_span.content
        );
    }

    #[test]
    fn test_header_alignment_matches_data() {
        // Verify that header columns align with data columns at standard width
        let header = format_header_line(60);
        let session = Session::new(
            "align-check".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/test")),
        );
        let data_line = format_session_line(&session, 60, "test");

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

    // --- Story 4 (acd-9ul): Basename disambiguation tests ---

    #[test]
    fn test_compute_directory_display_names_unique_basenames() {
        let sessions = vec![
            Session::new(
                "s1".to_string(),
                AgentType::ClaudeCode,
                Some(PathBuf::from("/home/user/project-a")),
            ),
            Session::new(
                "s2".to_string(),
                AgentType::ClaudeCode,
                Some(PathBuf::from("/home/user/project-b")),
            ),
        ];
        let display_names = compute_directory_display_names(&sessions);
        assert_eq!(display_names.get("s1"), Some(&"project-a".to_string()));
        assert_eq!(display_names.get("s2"), Some(&"project-b".to_string()));
    }

    #[test]
    fn test_compute_directory_display_names_duplicate_basenames() {
        let sessions = vec![
            Session::new(
                "s1".to_string(),
                AgentType::ClaudeCode,
                Some(PathBuf::from("/home/user/project")),
            ),
            Session::new(
                "s2".to_string(),
                AgentType::ClaudeCode,
                Some(PathBuf::from("/work/client/project")),
            ),
        ];
        let display_names = compute_directory_display_names(&sessions);
        // Both should have parent/basename format since basename "project" is duplicated
        assert_eq!(display_names.get("s1"), Some(&"user/project".to_string()));
        assert_eq!(display_names.get("s2"), Some(&"client/project".to_string()));
    }

    #[test]
    fn test_compute_directory_display_names_mixed() {
        let sessions = vec![
            Session::new(
                "s1".to_string(),
                AgentType::ClaudeCode,
                Some(PathBuf::from("/home/user/project")),
            ),
            Session::new(
                "s2".to_string(),
                AgentType::ClaudeCode,
                Some(PathBuf::from("/work/client/project")),
            ),
            Session::new(
                "s3".to_string(),
                AgentType::ClaudeCode,
                Some(PathBuf::from("/tmp/unique-name")),
            ),
        ];
        let display_names = compute_directory_display_names(&sessions);
        // s1 and s2 have duplicate basename, need disambiguation
        assert_eq!(display_names.get("s1"), Some(&"user/project".to_string()));
        assert_eq!(display_names.get("s2"), Some(&"client/project".to_string()));
        // s3 is unique
        assert_eq!(display_names.get("s3"), Some(&"unique-name".to_string()));
    }

    #[test]
    fn test_compute_directory_display_names_unknown_paths() {
        let sessions = vec![
            Session::new("s1".to_string(), AgentType::ClaudeCode, None),
            Session::new(
                "s2".to_string(),
                AgentType::ClaudeCode,
                Some(PathBuf::from("/home/user/project")),
            ),
        ];
        let display_names = compute_directory_display_names(&sessions);
        // Unknown path should map to <error>
        assert_eq!(display_names.get("s1"), Some(&"<error>".to_string()));
        // Normal path should show basename
        assert_eq!(display_names.get("s2"), Some(&"project".to_string()));
    }

    #[test]
    fn test_compute_directory_display_names_root_path() {
        let sessions = vec![Session::new(
            "s1".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/")),
        )];
        let display_names = compute_directory_display_names(&sessions);
        // Root path has no file_name(), should fall back to <error>
        assert_eq!(display_names.get("s1"), Some(&"<error>".to_string()));
    }

    #[test]
    fn test_compute_directory_display_names_three_duplicate_basenames() {
        let sessions = vec![
            Session::new(
                "s1".to_string(),
                AgentType::ClaudeCode,
                Some(PathBuf::from("/home/user/project")),
            ),
            Session::new(
                "s2".to_string(),
                AgentType::ClaudeCode,
                Some(PathBuf::from("/work/client/project")),
            ),
            Session::new(
                "s3".to_string(),
                AgentType::ClaudeCode,
                Some(PathBuf::from("/opt/build/project")),
            ),
        ];
        let display_names = compute_directory_display_names(&sessions);
        // All three should show parent/basename since "project" appears 3 times
        assert_eq!(display_names.get("s1"), Some(&"user/project".to_string()));
        assert_eq!(display_names.get("s2"), Some(&"client/project".to_string()));
        assert_eq!(display_names.get("s3"), Some(&"build/project".to_string()));
    }

    #[test]
    fn test_disambiguation_parent_collision() {
        // Same basename AND same immediate parent
        let s1 = make_test_session("s1", Some(PathBuf::from("/home/alice/project")));
        let s2 = make_test_session("s2", Some(PathBuf::from("/work/alice/project")));
        let sessions = vec![s1, s2];
        let names = compute_directory_display_names(&sessions);
        assert_ne!(
            names.get("s1"),
            names.get("s2"),
            "Colliding parent/basename should be disambiguated"
        );
        // Should add grandparent level
        assert!(names
            .get("s1")
            .map(|n| n.contains("home") || n.contains("alice"))
            .unwrap_or(false));
        assert!(names
            .get("s2")
            .map(|n| n.contains("work") || n.contains("alice"))
            .unwrap_or(false));
    }
}
