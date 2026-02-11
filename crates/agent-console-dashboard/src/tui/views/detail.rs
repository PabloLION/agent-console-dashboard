//! Session detail modal overlay view.
//!
//! Renders a centered modal showing comprehensive information about a single
//! session: status, working directory, session ID, API usage, and state
//! transition history. Supports scrolling through history entries.

use crate::{Session, Status};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::time::Instant;

/// Maximum history entries visible without scrolling.
const MAX_VISIBLE_HISTORY: usize = 5;

/// Renders the session detail modal overlay.
///
/// The modal is centered in the given `area` and displays session metadata,
/// API usage summary, state history (with scroll support), and action hints.
pub fn render_detail(
    frame: &mut Frame,
    session: &Session,
    area: Rect,
    history_scroll: usize,
    now: Instant,
) {
    let modal_width = 50u16.min(area.width.saturating_sub(4));
    let modal_height = 16u16.min(area.height.saturating_sub(2));

    if modal_width < 20 || modal_height < 8 {
        return; // Too small to render meaningfully
    }

    let x = area.x + (area.width.saturating_sub(modal_width)) / 2;
    let y = area.y + (area.height.saturating_sub(modal_height)) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    // Clear background
    frame.render_widget(Clear, modal_area);

    // Derive title from working directory basename or session ID
    let title = session
        .working_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&session.session_id);

    let block = Block::default()
        .title(format!("── {} ──", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let lines = build_detail_lines(session, inner.width, history_scroll, now, true);

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Renders the session detail as an inline panel below the session list.
///
/// Unlike `render_detail`, this renders into the given `area` directly
/// without clearing background or centering. Used for the non-modal layout
/// where detail appears as a fixed section below the session list.
pub fn render_inline_detail(
    frame: &mut Frame,
    session: &Session,
    area: Rect,
    history_scroll: usize,
    now: Instant,
) {
    if area.height < 3 || area.width < 20 {
        return; // Too small to render meaningfully
    }

    let title = session
        .working_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&session.session_id);

    let block = Block::default()
        .title(format!("── {} ──", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = build_detail_lines(session, inner.width, history_scroll, now, false);

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Renders a placeholder message when no session is selected.
pub fn render_detail_placeholder(frame: &mut Frame, area: Rect) {
    if area.height < 3 || area.width < 20 {
        return;
    }

    let block = Block::default()
        .title("── Detail ──")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let text = Paragraph::new(Line::from(vec![Span::styled(
        "Select a session to show detail",
        Style::default().fg(Color::DarkGray),
    )]));
    frame.render_widget(text, inner);
}

/// Builds the content lines for a detail view (shared between modal and inline).
///
/// When `show_actions` is true, footer action hints are appended (modal mode).
/// For inline mode, actions are omitted since keybindings are shown in the
/// main footer.
fn build_detail_lines<'a>(
    session: &'a Session,
    panel_width: u16,
    history_scroll: usize,
    now: Instant,
    show_actions: bool,
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line<'a>> = Vec::new();

    // Status line
    let elapsed = now.duration_since(session.since);
    let status_color = status_color(session.status);
    let elapsed_str = format_duration(elapsed.as_secs());
    lines.push(Line::from(vec![
        Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("{}", session.status),
            Style::default().fg(status_color),
        ),
        Span::raw(format!(" ({})", elapsed_str)),
    ]));

    // Working directory
    let wd = session.working_dir.display().to_string();
    let max_wd_len = (panel_width as usize).saturating_sub(13);
    let wd_display = if wd.len() > max_wd_len {
        format!("…{}", &wd[wd.len().saturating_sub(max_wd_len - 1)..])
    } else {
        wd
    };
    lines.push(Line::from(vec![
        Span::styled("Dir: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(wd_display),
    ]));

    // Session ID (truncated)
    let id_max = (panel_width as usize).saturating_sub(5);
    let id_display = if session.session_id.len() > id_max {
        format!("{}…", &session.session_id[..id_max.saturating_sub(1)])
    } else {
        session.session_id.clone()
    };
    lines.push(Line::from(vec![
        Span::styled("ID: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(id_display),
    ]));

    // API usage placeholder
    lines.push(Line::from(vec![
        Span::styled("Quota: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled("--", Style::default().fg(Color::DarkGray)),
    ]));

    // Blank separator
    lines.push(Line::raw(""));

    // History
    lines.push(Line::from(vec![Span::styled(
        "History:",
        Style::default().add_modifier(Modifier::BOLD),
    )]));

    if session.history.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  (no transitions)",
            Style::default().fg(Color::DarkGray),
        )]));
    } else {
        let total = session.history.len();
        let start = history_scroll.min(total.saturating_sub(MAX_VISIBLE_HISTORY));
        let end = (start + MAX_VISIBLE_HISTORY).min(total);

        // Show most recent first (reverse order)
        let reversed: Vec<_> = session.history.iter().rev().collect();
        let visible = &reversed[start..end];

        for transition in visible {
            let ts = format_transition_time(transition.timestamp, now);
            lines.push(Line::from(vec![
                Span::raw(format!("  {}  ", ts)),
                Span::styled(
                    format!("{}", transition.from),
                    Style::default().fg(status_color_for(transition.from)),
                ),
                Span::raw(" → "),
                Span::styled(
                    format!("{}", transition.to),
                    Style::default().fg(status_color_for(transition.to)),
                ),
            ]));
        }

        if total > MAX_VISIBLE_HISTORY {
            let indicator = format!("  [{}/{} entries]", end - start, total);
            lines.push(Line::from(vec![Span::styled(
                indicator,
                Style::default().fg(Color::DarkGray),
            )]));
        }
    }

    if show_actions {
        // Footer actions (modal mode only)
        let mut actions = vec![Span::styled(
            "[ESC] Back",
            Style::default().fg(Color::DarkGray),
        )];
        if session.status == Status::Closed {
            actions.insert(
                0,
                Span::styled("[R]esurrect  ", Style::default().fg(Color::Yellow)),
            );
        }
        actions.insert(
            actions.len() - 1,
            Span::styled("[C]lose  ", Style::default().fg(Color::Red)),
        );
        lines.push(Line::from(actions));
    }

    lines
}

/// Returns the display color for a session status.
fn status_color(status: Status) -> Color {
    status_color_for(status)
}

/// Maps a status to its display color.
fn status_color_for(status: Status) -> Color {
    match status {
        Status::Working => Color::Green,
        Status::Attention => Color::Yellow,
        Status::Question => Color::Magenta,
        Status::Closed => Color::DarkGray,
    }
}

/// Formats elapsed seconds as a human-readable duration string.
fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m{}s", secs / 60, secs % 60)
    } else {
        format!("{}h{}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Formats a transition timestamp relative to now as HH:MM:SS ago.
///
/// Since `Instant` doesn't map to wall-clock time, we display "Xm ago"
/// as a relative offset from now.
fn format_transition_time(timestamp: Instant, now: Instant) -> String {
    let ago = now.duration_since(timestamp).as_secs();
    if ago < 60 {
        format!("{:>3}s ago", ago)
    } else if ago < 3600 {
        format!("{:>2}m{:02}s ago", ago / 60, ago % 60)
    } else {
        format!("{:>2}h{:02}m ago", ago / 3600, (ago % 3600) / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentType, StateTransition};
    use std::path::PathBuf;
    use std::time::Duration;

    fn make_session(id: &str) -> Session {
        Session::new(
            id.to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/home/user/project-a"),
        )
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(0), "0s");
        assert_eq!(format_duration(45), "45s");
        assert_eq!(format_duration(59), "59s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(60), "1m0s");
        assert_eq!(format_duration(90), "1m30s");
        assert_eq!(format_duration(3599), "59m59s");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(3600), "1h0m");
        assert_eq!(format_duration(7260), "2h1m");
    }

    #[test]
    fn test_format_transition_time_seconds() {
        let now = Instant::now();
        let ts = now - Duration::from_secs(30);
        let result = format_transition_time(ts, now);
        assert!(result.contains("30s ago"));
    }

    #[test]
    fn test_format_transition_time_minutes() {
        let now = Instant::now();
        let ts = now - Duration::from_secs(150);
        let result = format_transition_time(ts, now);
        assert!(result.contains("2m30s ago"));
    }

    #[test]
    fn test_status_color_mapping() {
        assert_eq!(status_color_for(Status::Working), Color::Green);
        assert_eq!(status_color_for(Status::Attention), Color::Yellow);
        assert_eq!(status_color_for(Status::Question), Color::Magenta);
        assert_eq!(status_color_for(Status::Closed), Color::DarkGray);
    }

    #[test]
    fn test_render_detail_no_panic_normal() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let session = make_session("test-1");
        terminal
            .draw(|frame| {
                render_detail(frame, &session, frame.area(), 0, Instant::now());
            })
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_detail_no_panic_narrow() {
        let backend = ratatui::backend::TestBackend::new(25, 10);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let session = make_session("test-narrow");
        terminal
            .draw(|frame| {
                render_detail(frame, &session, frame.area(), 0, Instant::now());
            })
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_detail_no_panic_too_small() {
        let backend = ratatui::backend::TestBackend::new(10, 5);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let session = make_session("test-tiny");
        terminal
            .draw(|frame| {
                render_detail(frame, &session, frame.area(), 0, Instant::now());
            })
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_detail_with_history() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut session = make_session("test-history");
        let now = Instant::now();
        for i in 0..8 {
            session.history.push(StateTransition {
                timestamp: now - Duration::from_secs(60 * (8 - i)),
                from: Status::Working,
                to: Status::Attention,
                duration: Duration::from_secs(30),
            });
        }
        terminal
            .draw(|frame| {
                render_detail(frame, &session, frame.area(), 0, now);
            })
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_detail_closed_session() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut session = make_session("test-closed");
        session.status = Status::Closed;
        terminal
            .draw(|frame| {
                render_detail(frame, &session, frame.area(), 0, Instant::now());
            })
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_detail_long_working_dir() {
        let backend = ratatui::backend::TestBackend::new(60, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut session = make_session("test-long-wd");
        session.working_dir =
            PathBuf::from("/very/deeply/nested/path/to/some/project/directory/that/is/quite/long");
        terminal
            .draw(|frame| {
                render_detail(frame, &session, frame.area(), 0, Instant::now());
            })
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_detail_history_scroll() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut session = make_session("test-scroll");
        let now = Instant::now();
        for i in 0..10 {
            session.history.push(StateTransition {
                timestamp: now - Duration::from_secs(60 * (10 - i)),
                from: Status::Working,
                to: Status::Attention,
                duration: Duration::from_secs(30),
            });
        }
        // Scroll to offset 3
        terminal
            .draw(|frame| {
                render_detail(frame, &session, frame.area(), 3, now);
            })
            .expect("draw should not fail with scroll offset");
    }

    // --- render_inline_detail tests ---

    #[test]
    fn test_render_inline_detail_no_panic_normal() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let session = make_session("test-inline");
        terminal
            .draw(|frame| {
                render_inline_detail(frame, &session, frame.area(), 0, Instant::now());
            })
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_inline_detail_too_small_no_panic() {
        let backend = ratatui::backend::TestBackend::new(10, 2);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let session = make_session("test-tiny-inline");
        terminal
            .draw(|frame| {
                render_inline_detail(frame, &session, frame.area(), 0, Instant::now());
            })
            .expect("draw should not fail when too small");
    }

    #[test]
    fn test_render_inline_detail_with_history() {
        let backend = ratatui::backend::TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut session = make_session("test-inline-history");
        let now = Instant::now();
        for i in 0..6 {
            session.history.push(StateTransition {
                timestamp: now - Duration::from_secs(60 * (6 - i)),
                from: Status::Working,
                to: Status::Attention,
                duration: Duration::from_secs(30),
            });
        }
        terminal
            .draw(|frame| {
                render_inline_detail(frame, &session, frame.area(), 0, now);
            })
            .expect("draw should not fail");
    }

    // --- render_detail_placeholder tests ---

    #[test]
    fn test_render_detail_placeholder_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 10);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        terminal
            .draw(|frame| {
                render_detail_placeholder(frame, frame.area());
            })
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_detail_placeholder_too_small_no_panic() {
        let backend = ratatui::backend::TestBackend::new(10, 2);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        terminal
            .draw(|frame| {
                render_detail_placeholder(frame, frame.area());
            })
            .expect("draw should not fail when too small");
    }

    // --- build_detail_lines tests ---

    #[test]
    fn test_build_detail_lines_with_actions() {
        let session = make_session("test-lines");
        let lines = build_detail_lines(&session, 60, 0, Instant::now(), true);
        // Should contain status, dir, id, quota, blank, history header, history content, actions
        assert!(
            lines.len() >= 7,
            "expected at least 7 lines, got {}",
            lines.len()
        );
    }

    #[test]
    fn test_build_detail_lines_without_actions() {
        let session = make_session("test-lines-no-actions");
        let lines_with = build_detail_lines(&session, 60, 0, Instant::now(), true);
        let lines_without = build_detail_lines(&session, 60, 0, Instant::now(), false);
        // Without actions should have fewer lines (no action bar)
        assert!(
            lines_without.len() < lines_with.len(),
            "inline mode should have fewer lines than modal"
        );
    }
}
