//! Main rendering orchestration for the TUI dashboard.
//!
//! Provides the top-level `render_dashboard` function that composes
//! the header, session list, and footer into a cohesive layout.

use crate::tui::app::{App, View};
use crate::tui::views::dashboard::render_session_list;
use crate::tui::views::detail::{render_detail_placeholder, render_inline_detail};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::time::Instant;

/// Header text displayed at the top of the dashboard.
const HEADER_TEXT: &str = "Agent Console Dashboard";

/// Footer text showing available keybindings.
const FOOTER_TEXT: &str = "[j/k] Navigate  [Enter] Details  [r] Resurrect  [q] Quit";

/// Version string shown in the bottom-right corner.
const VERSION_TEXT: &str = concat!("v", env!("CARGO_PKG_VERSION"));

/// Renders the full dashboard layout: header, session list, optional detail, and footer.
///
/// When the detail view is active, the layout splits into four regions:
/// - Header: 1 line showing the application title
/// - Session list: flexible height (min 3 rows) showing all sessions
/// - Detail panel: fixed height showing selected session detail
/// - Footer: 1 line showing keybinding hints
///
/// When in dashboard-only mode, the detail panel is omitted and the session
/// list gets all available vertical space.
pub fn render_dashboard(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let now = Instant::now();

    let detail_active = matches!(app.view, View::Detail { .. });

    let chunks = if detail_active {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // header
                Constraint::Min(3),     // session list (minimum 3 rows)
                Constraint::Length(12), // detail panel
                Constraint::Length(1),  // footer
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // header
                Constraint::Min(1),    // session list
                Constraint::Length(1), // footer
            ])
            .split(area)
    };

    // Header
    let header = Paragraph::new(Line::from(vec![Span::styled(
        HEADER_TEXT,
        Style::default().fg(Color::Cyan),
    )]));
    frame.render_widget(header, chunks[0]);

    // Session list
    render_session_list(
        frame,
        chunks[1],
        &app.sessions,
        app.selected_index,
        area.width,
    );

    // Detail panel (only when detail view is active)
    if detail_active {
        if let View::Detail {
            session_index,
            history_scroll,
        } = app.view
        {
            if let Some(session) = app.sessions.get(session_index) {
                render_inline_detail(frame, session, chunks[2], history_scroll, now);
            } else {
                render_detail_placeholder(frame, chunks[2]);
            }
        }
    }

    // Footer (with optional status message overlay)
    let footer_idx = if detail_active { 3 } else { 2 };
    let footer_text = if let Some((ref msg, expiry)) = app.status_message {
        if Instant::now() < expiry {
            Line::from(vec![Span::styled(
                msg.clone(),
                Style::default().fg(Color::Yellow),
            )])
        } else {
            Line::from(vec![Span::styled(
                FOOTER_TEXT,
                Style::default().fg(Color::DarkGray),
            )])
        }
    } else {
        Line::from(vec![Span::styled(
            FOOTER_TEXT,
            Style::default().fg(Color::DarkGray),
        )])
    };
    let footer = Paragraph::new(footer_text);
    frame.render_widget(footer, chunks[footer_idx]);

    // Version string in the bottom-right corner (overlays footer area)
    let version = Paragraph::new(Line::from(vec![Span::styled(
        VERSION_TEXT,
        Style::default().fg(Color::DarkGray),
    )]))
    .alignment(Alignment::Right);
    frame.render_widget(version, chunks[footer_idx]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentType, Session, Status};
    use std::path::PathBuf;

    fn make_app() -> App {
        App::new(PathBuf::from("/tmp/test.sock"))
    }

    fn make_app_with_sessions(count: usize) -> App {
        let mut app = make_app();
        for i in 0..count {
            let mut session = Session::new(
                format!("session-{}", i),
                AgentType::ClaudeCode,
                Some(PathBuf::from(format!("/home/user/project-{}", i))),
            );
            if i % 4 == 1 {
                session.status = Status::Attention;
            } else if i % 4 == 2 {
                session.status = Status::Question;
            } else if i % 4 == 3 {
                session.status = Status::Closed;
            }
            app.sessions.push(session);
        }
        app
    }

    #[test]
    fn test_render_dashboard_empty_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let app = make_app();
        terminal
            .draw(|frame| render_dashboard(frame, &app))
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_dashboard_with_sessions_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut app = make_app_with_sessions(5);
        app.selected_index = Some(2);
        terminal
            .draw(|frame| render_dashboard(frame, &app))
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_dashboard_narrow_no_panic() {
        let backend = ratatui::backend::TestBackend::new(30, 10);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let app = make_app_with_sessions(3);
        terminal
            .draw(|frame| render_dashboard(frame, &app))
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_dashboard_wide_no_panic() {
        let backend = ratatui::backend::TestBackend::new(200, 50);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let app = make_app_with_sessions(10);
        terminal
            .draw(|frame| render_dashboard(frame, &app))
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_dashboard_minimal_height_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 3);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let app = make_app_with_sessions(5);
        terminal
            .draw(|frame| render_dashboard(frame, &app))
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_dashboard_single_row_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 1);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let app = make_app();
        terminal
            .draw(|frame| render_dashboard(frame, &app))
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_dashboard_many_sessions_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut app = make_app_with_sessions(100);
        app.selected_index = Some(50);
        terminal
            .draw(|frame| render_dashboard(frame, &app))
            .expect("draw should not fail");
    }

    #[test]
    fn test_header_text_content() {
        assert_eq!(HEADER_TEXT, "Agent Console Dashboard");
    }

    #[test]
    fn test_version_text_content() {
        assert!(VERSION_TEXT.starts_with("v"));
        assert!(VERSION_TEXT.contains("0.1.2"));
    }

    #[test]
    fn test_version_shown_in_footer_row() {
        let app = make_app_with_sessions(3);
        let buffer = render_dashboard_to_buffer(&app, 80, 24);
        let footer_row = buffer.area().height - 1;
        assert!(
            row_contains(&buffer, footer_row, VERSION_TEXT),
            "Footer row should contain version string"
        );
    }

    #[test]
    fn test_footer_text_content() {
        assert!(FOOTER_TEXT.contains("[j/k]"));
        assert!(FOOTER_TEXT.contains("[q] Quit"));
        assert!(FOOTER_TEXT.contains("[r] Resurrect"));
        assert!(FOOTER_TEXT.contains("[Enter] Details"));
    }

    // --- Detail view (inline panel) tests ---

    #[test]
    fn test_render_dashboard_with_detail_view_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 30);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut app = make_app_with_sessions(5);
        app.selected_index = Some(1);
        app.open_detail(1);
        terminal
            .draw(|frame| render_dashboard(frame, &app))
            .expect("draw should not fail with detail view active");
    }

    #[test]
    fn test_render_dashboard_detail_view_narrow_no_panic() {
        let backend = ratatui::backend::TestBackend::new(30, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut app = make_app_with_sessions(3);
        app.open_detail(0);
        terminal
            .draw(|frame| render_dashboard(frame, &app))
            .expect("draw should not fail with detail view on narrow terminal");
    }

    #[test]
    fn test_render_dashboard_detail_view_minimal_height_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 5);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut app = make_app_with_sessions(3);
        app.open_detail(0);
        terminal
            .draw(|frame| render_dashboard(frame, &app))
            .expect("draw should not fail with detail on minimal height");
    }

    #[test]
    fn test_render_dashboard_detail_view_out_of_bounds_session_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 30);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut app = make_app_with_sessions(2);
        // Force detail view with out-of-bounds index to test placeholder path
        app.view = View::Detail {
            session_index: 99,
            history_scroll: 0,
        };
        terminal
            .draw(|frame| render_dashboard(frame, &app))
            .expect("draw should not fail with out-of-bounds detail index");
    }

    // --- Full Dashboard Integration Tests (acd-211) ---

    use crate::tui::test_utils::{find_row_with_text, render_dashboard_to_buffer, row_contains};

    #[test]
    fn test_full_dashboard_render_with_mixed_statuses() {
        let mut app = App::new(PathBuf::from("/tmp/test.sock"));

        // Add 4 sessions with different statuses
        let mut s1 = Session::new(
            "working-session".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/work")),
        );
        s1.status = Status::Working;

        let mut s2 = Session::new(
            "attention-session".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/attention")),
        );
        s2.status = Status::Attention;

        let mut s3 = Session::new(
            "question-session".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/question")),
        );
        s3.status = Status::Question;

        let mut s4 = Session::new(
            "closed-session".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/closed")),
        );
        s4.status = Status::Closed;

        app.sessions.extend([s1, s2, s3, s4]);
        app.init_selection();

        let buffer = render_dashboard_to_buffer(&app, 80, 30);

        // Verify all sessions appear in the buffer
        assert!(
            find_row_with_text(&buffer, "working-session").is_some(),
            "Dashboard should show working session"
        );
        assert!(
            find_row_with_text(&buffer, "attention-session").is_some(),
            "Dashboard should show attention session"
        );
        assert!(
            find_row_with_text(&buffer, "question-session").is_some(),
            "Dashboard should show question session"
        );
        assert!(
            find_row_with_text(&buffer, "closed-session").is_some(),
            "Dashboard should show closed session"
        );

        // Verify structural elements
        assert!(
            find_row_with_text(&buffer, "Agent Console Dashboard").is_some(),
            "Should show header"
        );
        assert!(
            row_contains(&buffer, buffer.area().height - 1, "[q] Quit"),
            "Should show footer"
        );
    }

    #[test]
    fn test_full_dashboard_render_with_detail_panel() {
        let mut app = make_app_with_sessions(3);
        app.init_selection();
        app.open_detail(1);

        let buffer = render_dashboard_to_buffer(&app, 80, 35);

        // Verify session list is visible
        assert!(
            find_row_with_text(&buffer, "session-1").is_some(),
            "Session list should be visible"
        );

        // Verify detail panel is visible
        assert!(
            find_row_with_text(&buffer, "Status:").is_some(),
            "Detail panel should be visible with Status label"
        );

        assert!(
            find_row_with_text(&buffer, "Dir:").is_some(),
            "Detail panel should be visible with Dir label"
        );

        assert!(
            find_row_with_text(&buffer, "ID:").is_some(),
            "Detail panel should be visible with ID label"
        );

        // Verify both header and footer are present
        assert!(
            find_row_with_text(&buffer, "Agent Console Dashboard").is_some(),
            "Header should be visible"
        );
        assert!(
            row_contains(&buffer, buffer.area().height - 1, "[q] Quit"),
            "Footer should be visible"
        );
    }

    #[test]
    fn test_full_dashboard_render_many_sessions_scrolling() {
        let mut app = App::new(PathBuf::from("/tmp/test.sock"));

        // Add 50 sessions
        for i in 0..50 {
            let mut session = Session::new(
                format!("session-{:02}", i),
                AgentType::ClaudeCode,
                Some(PathBuf::from(format!("/tmp/project-{}", i))),
            );
            if i % 4 == 1 {
                session.status = Status::Attention;
            } else if i % 4 == 2 {
                session.status = Status::Question;
            } else if i % 4 == 3 {
                session.status = Status::Closed;
            }
            app.sessions.push(session);
        }

        // Select session #25
        app.selected_index = Some(25);

        let buffer = render_dashboard_to_buffer(&app, 100, 40);

        // The selected session should appear in the buffer
        // (ratatui's List widget handles scrolling automatically to show selection)
        assert!(
            find_row_with_text(&buffer, "session-25").is_some(),
            "Selected session should be visible in scrolled view"
        );

        // Verify structural integrity with many sessions
        assert!(
            find_row_with_text(&buffer, "Agent Console Dashboard").is_some(),
            "Header should be visible with many sessions"
        );
        assert!(
            find_row_with_text(&buffer, "Sessions").is_some(),
            "Session border should be visible"
        );
        assert!(
            row_contains(&buffer, buffer.area().height - 1, "[q] Quit"),
            "Footer should be visible with many sessions"
        );
    }
}
