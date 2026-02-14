use super::*;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use serial_test::serial;

fn make_mouse_event(kind: MouseEventKind, row: u16, column: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: crossterm::event::KeyModifiers::NONE,
    }
}

#[test]
#[serial]
fn test_calculate_clicked_session_valid_row() {
    use crate::tui::test_utils::render_dashboard_to_buffer;
    let mut app = make_app_with_sessions(5);
    // Render to populate session_list_inner_area
    let _ = render_dashboard_to_buffer(&mut app, 80, 24);

    // Now calculate_clicked_session uses the stored inner_area
    // In normal mode (80x24): header=0, session_list starts at row 1,
    // has column header + border, inner starts at row 3
    assert_eq!(app.calculate_clicked_session(3), Some(0));
    assert_eq!(app.calculate_clicked_session(4), Some(1));
    assert_eq!(app.calculate_clicked_session(7), Some(4));
}

#[test]
#[serial]
fn test_calculate_clicked_session_header_returns_none() {
    use crate::tui::test_utils::render_dashboard_to_buffer;
    let mut app = make_app_with_sessions(5);
    // Render to populate session_list_inner_area
    let _ = render_dashboard_to_buffer(&mut app, 80, 24);

    // Clicks above inner_area (header, column header, border) return None
    assert_eq!(app.calculate_clicked_session(0), None);
    assert_eq!(app.calculate_clicked_session(1), None);
    assert_eq!(app.calculate_clicked_session(2), None);
}

#[test]
#[serial]
fn test_calculate_clicked_session_out_of_bounds() {
    use crate::tui::test_utils::render_dashboard_to_buffer;
    let mut app = make_app_with_sessions(3);
    // Render to populate session_list_inner_area
    let _ = render_dashboard_to_buffer(&mut app, 80, 24);

    // Sessions at rows 3, 4, 5 (indices 0, 1, 2) - click beyond returns None
    assert_eq!(app.calculate_clicked_session(6), None);
    assert_eq!(app.calculate_clicked_session(10), None);
}

#[test]
fn test_mouse_left_click_selects_and_opens_detail() {
    use crate::tui::test_utils::render_dashboard_to_buffer;
    let mut app = make_app_with_sessions(5);
    app.selected_index = Some(0);
    // Render to populate session_list_inner_area
    let _ = render_dashboard_to_buffer(&mut app, 80, 24);

    // In normal mode: inner starts at row 3, so row 5 = session index 2
    let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 5, 10);
    let action = app.handle_mouse_event(mouse);
    assert_eq!(action, Action::None);
    assert_eq!(app.selected_index, Some(2));
    // Single click focuses the session (detail panel updates automatically)
    // View stays as Dashboard (detail is always visible)
    assert_eq!(app.view, View::Dashboard);
}

#[test]
fn test_mouse_left_click_header_clears_selection() {
    let mut app = make_app_with_sessions(3);
    app.selected_index = Some(1);
    let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 0, 10);
    let action = app.handle_mouse_event(mouse);
    assert_eq!(action, Action::None);
    assert_eq!(
        app.selected_index, None,
        "Header click should clear selection"
    );
    assert_eq!(
        app.view,
        View::Dashboard,
        "Header click should close detail view"
    );
}

#[test]
fn test_mouse_header_click_from_detail_view_returns_to_dashboard() {
    let mut app = make_app_with_sessions(3);
    app.selected_index = Some(1);
    // open_detail is now deprecated (no-op), but call it for backward compat
    app.open_detail(1);
    // View is always Dashboard now (detail panel is always visible)
    assert_eq!(app.view, View::Dashboard, "View is always Dashboard");
    // Click header
    let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 0, 10);
    let action = app.handle_mouse_event(mouse);
    assert_eq!(action, Action::None);
    assert_eq!(
        app.selected_index, None,
        "Header click should clear selection (defocus)"
    );
    assert_eq!(app.view, View::Dashboard, "View stays as Dashboard");
}

#[test]
fn test_initial_state_no_selection() {
    let app = App::new(PathBuf::from("/tmp/test.sock"));
    assert_eq!(
        app.selected_index, None,
        "Initial state should have no selection"
    );
}

#[test]
fn test_mouse_double_click_fires_hook_returns_none() {
    use crate::tui::test_utils::render_dashboard_to_buffer;
    let mut app = make_app_with_sessions(3);
    // Render to populate session_list_inner_area
    let _ = render_dashboard_to_buffer(&mut app, 80, 24);

    // First click: focuses the session (row 4 = session index 1)
    let mouse1 = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 4, 10);
    let action1 = app.handle_mouse_event(mouse1);
    assert_eq!(action1, Action::None);
    assert_eq!(app.selected_index, Some(1));
    // View is always Dashboard (detail panel is always visible)
    assert_eq!(app.view, View::Dashboard);

    // Second click in quick succession at same position (double-click)
    let mouse2 = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 4, 10);
    let action2 = app.handle_mouse_event(mouse2);
    assert_eq!(action2, Action::None);
    assert!(app.last_click.is_none());
}

#[test]
fn test_mouse_double_click_different_position_no_detail() {
    use crate::tui::test_utils::render_dashboard_to_buffer;
    let mut app = make_app_with_sessions(5);
    // Render to populate session_list_inner_area
    let _ = render_dashboard_to_buffer(&mut app, 80, 24);

    // First click (row 3 = session index 0)
    let mouse1 = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 3, 10);
    app.handle_mouse_event(mouse1);

    // Second click at different row (row 5 = session index 2)
    let mouse2 = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 5, 10);
    let action2 = app.handle_mouse_event(mouse2);
    assert_eq!(action2, Action::None);
    assert_eq!(app.selected_index, Some(2));
}

#[test]
fn test_mouse_scroll_down_selects_next() {
    let mut app = make_app_with_sessions(5);
    app.selected_index = Some(1);
    let mouse = make_mouse_event(MouseEventKind::ScrollDown, 5, 10);
    let action = app.handle_mouse_event(mouse);
    assert_eq!(action, Action::None);
    assert_eq!(app.selected_index, Some(2));
}

#[test]
fn test_mouse_scroll_up_selects_previous() {
    let mut app = make_app_with_sessions(5);
    app.selected_index = Some(2);
    let mouse = make_mouse_event(MouseEventKind::ScrollUp, 5, 10);
    let action = app.handle_mouse_event(mouse);
    assert_eq!(action, Action::None);
    assert_eq!(app.selected_index, Some(1));
}

#[test]
fn test_mouse_scroll_at_boundaries() {
    let mut app = make_app_with_sessions(3);
    // Scroll down at end
    app.selected_index = Some(2);
    let mouse_down = make_mouse_event(MouseEventKind::ScrollDown, 5, 10);
    app.handle_mouse_event(mouse_down);
    assert_eq!(app.selected_index, Some(2));

    // Scroll up at start
    app.selected_index = Some(0);
    let mouse_up = make_mouse_event(MouseEventKind::ScrollUp, 5, 10);
    app.handle_mouse_event(mouse_up);
    assert_eq!(app.selected_index, Some(0));
}

#[test]
fn test_mouse_click_in_detail_view_reselects() {
    use crate::tui::test_utils::render_dashboard_to_buffer;
    let mut app = make_app_with_sessions(3);
    // open_detail is deprecated (no-op)
    app.open_detail(0);
    app.selected_index = Some(0);
    // Render to populate session_list_inner_area
    let _ = render_dashboard_to_buffer(&mut app, 80, 24);

    // Click row 4 (session index 1)
    let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 4, 10);
    let action = app.handle_mouse_event(mouse);
    assert_eq!(action, Action::None);
    assert_eq!(app.selected_index, Some(1));
    // View is always Dashboard (detail is always visible)
    assert_eq!(app.view, View::Dashboard);
}

#[test]
fn test_mouse_scroll_in_detail_view_scrolls_history() {
    let mut app = make_app_with_sessions(1);
    for _ in 0..10 {
        app.sessions[0].history.push(crate::StateTransition {
            timestamp: std::time::Instant::now(),
            from: crate::Status::Working,
            to: crate::Status::Attention,
            duration: std::time::Duration::from_secs(1),
        });
    }
    app.selected_index = Some(0);

    // Scroll now always navigates sessions (detail panel never steals focus)
    let scroll = make_mouse_event(MouseEventKind::ScrollDown, 5, 10);
    let action = app.handle_mouse_event(scroll);
    assert_eq!(action, Action::None);
    // Scroll down tries to select next session (clamped at boundary)
    assert_eq!(app.selected_index, Some(0));
    // View is always Dashboard
    assert_eq!(app.view, View::Dashboard);
}

#[test]
fn test_mouse_right_click_ignored() {
    use crate::tui::test_utils::render_dashboard_to_buffer;
    let mut app = make_app_with_sessions(3);
    app.selected_index = Some(0);
    // Render to populate session_list_inner_area
    let _ = render_dashboard_to_buffer(&mut app, 80, 24);

    let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Right), 3, 10);
    let action = app.handle_mouse_event(mouse);
    assert_eq!(action, Action::None);
    assert_eq!(app.selected_index, Some(0));
}

#[test]
fn test_last_click_initialized_to_none() {
    let app = App::new(PathBuf::from("/tmp/test.sock"));
    assert!(app.last_click.is_none());
}

#[test]
fn test_double_click_hook_default_none() {
    let app = App::new(PathBuf::from("/tmp/test.sock"));
    assert!(app.double_click_hook.is_none());
}

#[test]
fn test_status_message_default_none() {
    let app = App::new(PathBuf::from("/tmp/test.sock"));
    assert!(app.status_message.is_none());
}

#[test]
fn test_double_click_no_hook_sets_config_message() {
    use crate::tui::test_utils::render_dashboard_to_buffer;
    let mut app = make_app_with_sessions(3);
    app.double_click_hook = None;
    // Render to populate session_list_inner_area
    let _ = render_dashboard_to_buffer(&mut app, 80, 24);

    let first_click = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 3, 10);
    app.handle_mouse_event(first_click);

    let second_click = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 3, 10);
    app.handle_mouse_event(second_click);

    assert!(app.status_message.is_some(), "should set status message");
    let (msg, _) = app.status_message.as_ref().expect("msg");
    assert!(
        msg.contains("double_click_hook"),
        "message should mention config key"
    );
}

#[test]
fn test_double_click_with_hook_sets_confirmation() {
    use crate::tui::test_utils::render_dashboard_to_buffer;
    let mut app = make_app_with_sessions(3);
    app.double_click_hook = Some("echo test".to_string());
    // Render to populate session_list_inner_area
    let _ = render_dashboard_to_buffer(&mut app, 80, 24);

    let first_click = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 3, 10);
    app.handle_mouse_event(first_click);

    let second_click = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 3, 10);
    app.handle_mouse_event(second_click);

    assert!(app.status_message.is_some(), "should set status message");
    let (msg, _) = app.status_message.as_ref().expect("msg");
    assert_eq!(msg, "Hook executed");
}

#[test]
fn test_expire_status_message_clears_expired() {
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));
    app.status_message = Some((
        "old message".to_string(),
        Instant::now() - Duration::from_secs(1),
    ));
    app.expire_status_message();
    assert!(
        app.status_message.is_none(),
        "expired message should be cleared"
    );
}

#[test]
fn test_expire_status_message_keeps_fresh() {
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));
    app.status_message = Some((
        "fresh message".to_string(),
        Instant::now() + Duration::from_secs(10),
    ));
    app.expire_status_message();
    assert!(app.status_message.is_some(), "fresh message should be kept");
}

// --- substitute_hook_placeholders tests ---

#[test]
fn test_substitute_hook_all_placeholders() {
    let session = Session::new(
        "sess-123".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/home/user/project")),
    );
    let result = substitute_hook_placeholders("open {working_dir} --id={session_id}", &session);
    assert_eq!(result, "open /home/user/project --id=sess-123");
}

#[test]
fn test_substitute_hook_status_placeholder() {
    let mut session = Session::new(
        "sess-456".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp")),
    );
    session.status = crate::Status::Attention;
    let result = substitute_hook_placeholders("echo {status}", &session);
    assert_eq!(result, "echo attention");
}

#[test]
fn test_substitute_hook_no_placeholders() {
    let session = Session::new(
        "sess-789".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp")),
    );
    let result = substitute_hook_placeholders("echo hello", &session);
    assert_eq!(result, "echo hello");
}

#[test]
fn test_substitute_hook_repeated_placeholders() {
    let session = Session::new(
        "abc".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/x")),
    );
    let result = substitute_hook_placeholders("{session_id} and {session_id}", &session);
    assert_eq!(result, "abc and abc");
}

#[test]
fn test_substitute_hook_empty_template() {
    let session = Session::new(
        "s".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/")),
    );
    let result = substitute_hook_placeholders("", &session);
    assert_eq!(result, "");
}

// --- SessionSnapshot stdin tests ---

#[test]
fn test_execute_double_click_hook_serializes_session_snapshot() {
    use crate::SessionSnapshot;

    // Create a session with known values
    let mut session = Session::new(
        "test-session-123".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp/test-project")),
    );
    session.status = Status::Attention;

    // Convert to SessionSnapshot and serialize
    let snapshot: SessionSnapshot = (&session).into();
    let json_str = serde_json::to_string(&snapshot).expect("should serialize");

    // Verify the JSON can be deserialized back to SessionSnapshot
    let parsed: SessionSnapshot =
        serde_json::from_str(&json_str).expect("Should be valid SessionSnapshot JSON");

    assert_eq!(parsed.session_id, "test-session-123");
    assert_eq!(parsed.status, "attention");
    assert_eq!(parsed.working_dir, Some("/tmp/test-project".to_string()));
    assert_eq!(parsed.agent_type, "claudecode");
    assert!(!parsed.closed);
}

// --- Mouse Interaction Tests (acd-211) ---

use crate::tui::test_utils::{find_row_with_text, render_dashboard_to_buffer};
use ratatui::style::Color;

#[test]
fn test_click_selects_and_renders_highlight() {
    let mut app = make_app_with_sessions(5);
    app.selected_index = Some(0);

    // Render first to populate session_list_inner_area
    let _ = render_dashboard_to_buffer(&mut app, 80, 30);

    // Click session at row 5 (session index 2)
    let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 5, 10);
    app.handle_mouse_event(mouse);

    assert_eq!(
        app.selected_index,
        Some(2),
        "Click should select session at index 2"
    );

    let buffer = render_dashboard_to_buffer(&mut app, 80, 30);

    let session_row = find_row_with_text(&buffer, "session-2").expect("should find session-2");

    let mut found_highlight = false;
    for col in 0..buffer.area().width {
        if let Some(cell) = buffer.cell((col, session_row)) {
            if cell.bg == Color::DarkGray {
                found_highlight = true;
                break;
            }
        }
    }

    assert!(
        found_highlight,
        "Clicked session should have highlight in rendered buffer"
    );
}

#[test]
fn test_no_selection_renders_no_highlight() {
    let mut app = make_app_with_sessions(3);
    app.selected_index = None;

    let buffer = render_dashboard_to_buffer(&mut app, 80, 30);

    for row in 0..buffer.area().height {
        let mut found_highlight_in_row = false;
        for col in 0..buffer.area().width {
            if let Some(cell) = buffer.cell((col, row)) {
                if cell.bg == Color::DarkGray {
                    found_highlight_in_row = true;
                    break;
                }
            }
        }
        assert!(
            !found_highlight_in_row,
            "No row should have highlight when selected_index is None, but row {} has DarkGray background",
            row
        );
    }
}

// --- Rect-Based Click Detection Tests (acd-khj) ---

#[test]
fn test_click_detection_works_in_normal_mode() {
    use crate::tui::test_utils::render_dashboard_to_buffer;
    let mut app = make_app_with_sessions(3);
    // Normal mode: 80x24 terminal
    let _ = render_dashboard_to_buffer(&mut app, 80, 24);

    // Verify session_list_inner_area is populated
    assert!(
        app.session_list_inner_area.is_some(),
        "session_list_inner_area should be set after render"
    );

    // Click first session - should work
    let inner = app.session_list_inner_area.unwrap();
    let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Left), inner.y, 10);
    app.handle_mouse_event(mouse);
    assert_eq!(app.selected_index, Some(0));
}

#[test]
fn test_click_detection_works_in_narrow_mode() {
    use crate::tui::test_utils::render_dashboard_to_buffer;
    let mut app = make_app_with_sessions(3);
    // Narrow mode: width < 40
    let _ = render_dashboard_to_buffer(&mut app, 30, 24);

    // Verify session_list_inner_area is populated
    assert!(
        app.session_list_inner_area.is_some(),
        "session_list_inner_area should be set in narrow mode"
    );

    // Click first session - should work in narrow mode too
    let inner = app.session_list_inner_area.unwrap();
    let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Left), inner.y, 10);
    app.handle_mouse_event(mouse);
    assert_eq!(app.selected_index, Some(0));
}

#[test]
fn test_click_detection_returns_none_without_render() {
    let mut app = make_app_with_sessions(3);
    // Don't render - session_list_inner_area is None

    // Click should return None because inner_area is not set
    let result = app.calculate_clicked_session(3);
    assert_eq!(result, None, "Should return None when session_list_inner_area is None");
}

#[test]
#[serial]
fn test_click_detection_works_in_debug_mode() {
    use crate::tui::test_utils::render_dashboard_to_buffer;
    // Enable debug ruler
    std::env::set_var("AGENT_CONSOLE_DASHBOARD_DEBUG", "1");

    let mut app = make_app_with_sessions(3);
    // Render with debug mode enabled
    let _ = render_dashboard_to_buffer(&mut app, 80, 24);

    // Verify session_list_inner_area is populated
    assert!(
        app.session_list_inner_area.is_some(),
        "session_list_inner_area should be set in debug mode"
    );

    // Click first session - should work in debug mode with ruler
    let inner = app.session_list_inner_area.unwrap();
    let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Left), inner.y, 10);
    app.handle_mouse_event(mouse);
    assert_eq!(app.selected_index, Some(0));

    // Clean up env var
    std::env::remove_var("AGENT_CONSOLE_DASHBOARD_DEBUG");
}
