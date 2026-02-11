//! Main rendering orchestration for the TUI dashboard.
//!
//! Provides the top-level `render_dashboard` function that composes
//! the header, session list, and footer into a cohesive layout.

use crate::tui::app::{App, View};
use crate::tui::views::dashboard::render_session_list;
use crate::tui::views::detail::{render_detail_placeholder, render_inline_detail};
use ratatui::{
    layout::{Constraint, Direction, Layout},
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

    // Footer
    let footer_idx = if detail_active { 3 } else { 2 };
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        FOOTER_TEXT,
        Style::default().fg(Color::DarkGray),
    )]));
    frame.render_widget(footer, chunks[footer_idx]);
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
}
