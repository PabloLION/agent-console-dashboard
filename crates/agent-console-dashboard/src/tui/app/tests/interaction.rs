use super::*;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

fn make_mouse_event(kind: MouseEventKind, row: u16, column: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: crossterm::event::KeyModifiers::NONE,
    }
}

/// Sets up an app with a known inner area for click detection tests.
/// Simulates normal mode layout where sessions start at row 3.
fn make_clickable_app(session_count: usize) -> App {
    let mut app = make_app_with_sessions(session_count);
    // Normal mode: header(0) + column_header(1) + block_border(2) → sessions start at row 3
    app.session_list_inner_area = Some(Rect::new(0, 3, 80, 20));
    app
}

// --- calculate_clicked_session unit tests ---

#[test]
fn test_calculate_clicked_session_valid_row() {
    let app = make_clickable_app(5);
    assert_eq!(app.calculate_clicked_session(3), Some(0));
    assert_eq!(app.calculate_clicked_session(4), Some(1));
    assert_eq!(app.calculate_clicked_session(7), Some(4));
}

#[test]
fn test_calculate_clicked_session_header_returns_none() {
    let app = make_clickable_app(5);
    assert_eq!(app.calculate_clicked_session(0), None);
    assert_eq!(app.calculate_clicked_session(1), None);
    assert_eq!(app.calculate_clicked_session(2), None);
}

#[test]
fn test_calculate_clicked_session_out_of_bounds() {
    let app = make_clickable_app(3);
    // Sessions at rows 3, 4, 5 (indices 0, 1, 2) — row 6+ is out of bounds
    assert_eq!(app.calculate_clicked_session(6), None);
    assert_eq!(app.calculate_clicked_session(10), None);
}

#[test]
fn test_calculate_clicked_session_no_inner_area() {
    let app = make_app_with_sessions(3);
    // No render → session_list_inner_area is None → always returns None
    assert_eq!(app.calculate_clicked_session(3), None);
}

#[test]
fn test_calculate_clicked_session_different_offset() {
    // Simulates debug mode where sessions start at row 4
    let mut app = make_app_with_sessions(3);
    app.session_list_inner_area = Some(Rect::new(0, 4, 80, 20));
    assert_eq!(app.calculate_clicked_session(3), None); // above inner area
    assert_eq!(app.calculate_clicked_session(4), Some(0));
    assert_eq!(app.calculate_clicked_session(5), Some(1));
}

#[test]
fn test_calculate_clicked_session_narrow_mode() {
    // Simulates narrow mode where sessions start at row 2 (no column header)
    let mut app = make_app_with_sessions(3);
    app.session_list_inner_area = Some(Rect::new(0, 2, 30, 20));
    assert_eq!(app.calculate_clicked_session(1), None);
    assert_eq!(app.calculate_clicked_session(2), Some(0));
    assert_eq!(app.calculate_clicked_session(3), Some(1));
}

// --- Mouse event handler tests ---

#[test]
fn test_mouse_left_click_selects_session() {
    let mut app = make_clickable_app(5);
    app.selected_index = Some(0);
    // Row 5 → session index 2
    let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 5, 10);
    let action = app.handle_mouse_event(mouse);
    assert_eq!(action, Action::None);
    assert_eq!(app.selected_index, Some(2));
    assert_eq!(app.view, View::Dashboard);
}

#[test]
fn test_mouse_left_click_header_clears_selection() {
    let mut app = make_clickable_app(3);
    app.selected_index = Some(1);
    let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 0, 10);
    let action = app.handle_mouse_event(mouse);
    assert_eq!(action, Action::None);
    assert_eq!(
        app.selected_index, None,
        "Header click should clear selection"
    );
    assert_eq!(app.view, View::Dashboard);
}

#[test]
fn test_mouse_header_click_from_detail_view_returns_to_dashboard() {
    let mut app = make_clickable_app(3);
    app.selected_index = Some(1);
    app.open_detail(1);
    assert_eq!(app.view, View::Dashboard, "View is always Dashboard");
    let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 0, 10);
    let action = app.handle_mouse_event(mouse);
    assert_eq!(action, Action::None);
    assert_eq!(
        app.selected_index, None,
        "Header click should clear selection"
    );
    assert_eq!(app.view, View::Dashboard);
}

#[test]
fn test_initial_state_no_selection() {
    let app = App::new(PathBuf::from("/tmp/test.sock"), None);
    assert_eq!(app.selected_index, None);
}

#[test]
fn test_mouse_double_click_fires_hook() {
    let mut app = make_clickable_app(3);
    // First click: row 4 → session index 1
    let mouse1 = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 4, 10);
    app.handle_mouse_event(mouse1);
    assert_eq!(app.selected_index, Some(1));

    // Second click at same position (double-click)
    let mouse2 = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 4, 10);
    let action2 = app.handle_mouse_event(mouse2);
    assert_eq!(action2, Action::None);
    assert!(app.last_click.is_none());
}

#[test]
fn test_mouse_double_click_different_position() {
    let mut app = make_clickable_app(5);
    // First click
    let mouse1 = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 3, 10);
    app.handle_mouse_event(mouse1);
    // Second click at different row
    let mouse2 = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 5, 10);
    app.handle_mouse_event(mouse2);
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
    app.selected_index = Some(2);
    let mouse_down = make_mouse_event(MouseEventKind::ScrollDown, 5, 10);
    app.handle_mouse_event(mouse_down);
    assert_eq!(app.selected_index, Some(2));

    app.selected_index = Some(0);
    let mouse_up = make_mouse_event(MouseEventKind::ScrollUp, 5, 10);
    app.handle_mouse_event(mouse_up);
    assert_eq!(app.selected_index, Some(0));
}

#[test]
fn test_mouse_click_reselects() {
    let mut app = make_clickable_app(3);
    app.open_detail(0);
    app.selected_index = Some(0);
    // Row 4 → session index 1
    let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 4, 10);
    let action = app.handle_mouse_event(mouse);
    assert_eq!(action, Action::None);
    assert_eq!(app.selected_index, Some(1));
    assert_eq!(app.view, View::Dashboard);
}

#[test]
fn test_mouse_scroll_navigates_sessions() {
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
    let scroll = make_mouse_event(MouseEventKind::ScrollDown, 5, 10);
    let action = app.handle_mouse_event(scroll);
    assert_eq!(action, Action::None);
    assert_eq!(app.selected_index, Some(0)); // clamped at boundary
    assert_eq!(app.view, View::Dashboard);
}

#[test]
fn test_mouse_right_click_ignored() {
    let mut app = make_clickable_app(3);
    app.selected_index = Some(0);
    let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Right), 3, 10);
    let action = app.handle_mouse_event(mouse);
    assert_eq!(action, Action::None);
    assert_eq!(app.selected_index, Some(0));
}

#[test]
fn test_last_click_initialized_to_none() {
    let app = App::new(PathBuf::from("/tmp/test.sock"), None);
    assert!(app.last_click.is_none());
}

#[test]
fn test_activate_hook_default_none() {
    let app = App::new(PathBuf::from("/tmp/test.sock"), None);
    assert!(app.activate_hook.is_none());
}

#[test]
fn test_reopen_hook_default_none() {
    let app = App::new(PathBuf::from("/tmp/test.sock"), None);
    assert!(app.reopen_hook.is_none());
}

#[test]
fn test_status_message_default_none() {
    let app = App::new(PathBuf::from("/tmp/test.sock"), None);
    assert!(app.status_message.is_none());
}

#[test]
fn test_double_click_no_hook_sets_config_message() {
    let mut app = make_clickable_app(3);
    app.activate_hook = None;
    let first_click = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 3, 10);
    app.handle_mouse_event(first_click);
    let second_click = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 3, 10);
    app.handle_mouse_event(second_click);
    assert!(app.status_message.is_some(), "should set status message");
    let (msg, _) = app.status_message.as_ref().expect("msg");
    assert!(
        msg.contains("activate_hook"),
        "message should mention config key"
    );
}

#[test]
fn test_double_click_with_activate_hook() {
    let mut app = make_clickable_app(3);
    app.activate_hook = Some("echo test".to_string());
    let first_click = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 3, 10);
    app.handle_mouse_event(first_click);
    let second_click = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 3, 10);
    app.handle_mouse_event(second_click);
    assert!(app.status_message.is_some(), "should set status message");
    let (msg, _) = app.status_message.as_ref().expect("msg");
    assert_eq!(msg, "Hook executed");
}

#[test]
fn test_double_click_closed_session_fires_reopen_hook() {
    let mut app = make_clickable_app(3);
    app.sessions[0].status = Status::Closed;
    app.reopen_hook = Some("echo reopen".to_string());
    let first_click = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 3, 10);
    app.handle_mouse_event(first_click);
    let second_click = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 3, 10);
    app.handle_mouse_event(second_click);
    assert!(app.status_message.is_some(), "should set status message");
    let (msg, _) = app.status_message.as_ref().expect("msg");
    assert_eq!(msg, "Hook executed");
    // Session should be updated to Attention locally
    assert_eq!(app.sessions[0].status, Status::Attention);
}

#[test]
fn test_double_click_closed_session_no_reopen_hook() {
    let mut app = make_clickable_app(3);
    app.sessions[0].status = Status::Closed;
    app.reopen_hook = None;
    let first_click = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 3, 10);
    app.handle_mouse_event(first_click);
    let second_click = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 3, 10);
    app.handle_mouse_event(second_click);
    assert!(app.status_message.is_some(), "should set status message");
    let (msg, _) = app.status_message.as_ref().expect("msg");
    assert!(
        msg.contains("reopen_hook"),
        "message should mention reopen_hook"
    );
    // Session should remain closed
    assert_eq!(app.sessions[0].status, Status::Closed);
}

#[test]
fn test_expire_status_message_clears_expired() {
    let mut app = App::new(PathBuf::from("/tmp/test.sock"), None);
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
    let mut app = App::new(PathBuf::from("/tmp/test.sock"), None);
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

// --- SessionSnapshot conversion test ---

#[test]
fn test_session_snapshot_conversion() {
    use crate::SessionSnapshot;

    let mut session = Session::new(
        "test-session-123".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp/test-project")),
    );
    session.status = Status::Attention;

    let snapshot: SessionSnapshot = (&session).into();
    assert_eq!(snapshot.session_id, "test-session-123");
    assert_eq!(snapshot.status, "attention");
    assert_eq!(snapshot.working_dir, Some("/tmp/test-project".to_string()));
    assert_eq!(snapshot.agent_type, "claudecode");
    assert!(!snapshot.closed);
}

// --- TwoLine layout mouse interaction tests ---

fn make_two_line_app(session_count: usize, terminal_width: u16) -> App {
    let mut app = App::new(
        PathBuf::from("/tmp/test.sock"),
        Some(crate::tui::app::LayoutMode::TwoLine),
    );
    for i in 0..session_count {
        app.sessions.push(Session::new(
            format!("session-{}", i),
            AgentType::ClaudeCode,
            Some(PathBuf::from(format!("/home/user/project-{}", i))),
        ));
    }
    app.init_selection();
    app.terminal_width = terminal_width;
    app
}

#[test]
fn test_two_line_click_chip_selects() {
    let mut app = make_two_line_app(10, 80);
    app.selected_index = Some(0);
    app.compact_scroll_offset = 0;

    // Click on second chip (column 25, within second chip area)
    let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 0, 25);
    let action = app.handle_mouse_event(mouse);
    assert_eq!(action, Action::None);
    assert_eq!(app.selected_index, Some(1), "should select chip at index 1");
}

#[test]
fn test_two_line_click_left_overflow_scrolls_and_focuses() {
    let mut app = make_two_line_app(10, 80);
    app.compact_scroll_offset = 5;
    app.selected_index = Some(5);

    // Click on left overflow indicator (column < 7)
    let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 0, 3);
    let action = app.handle_mouse_event(mouse);
    assert_eq!(action, Action::None);
    assert_eq!(
        app.compact_scroll_offset, 4,
        "should scroll left by 1"
    );
    assert_eq!(
        app.selected_index,
        Some(4),
        "should focus new leftmost chip"
    );
}

#[test]
fn test_two_line_click_right_overflow_scrolls_and_focuses() {
    let mut app = make_two_line_app(10, 80);
    app.compact_scroll_offset = 0;
    app.selected_index = Some(0);

    // Click on right overflow indicator (column > content_end)
    // Content ends at 7 + (3 chips * 18) = 61
    let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 0, 65);
    let action = app.handle_mouse_event(mouse);
    assert_eq!(action, Action::None);
    assert_eq!(
        app.compact_scroll_offset, 1,
        "should scroll right by 1"
    );
    // Rightmost visible chip: offset(1) + max_visible(3) - 1 = 3
    assert_eq!(
        app.selected_index,
        Some(3),
        "should focus new rightmost chip"
    );
}

#[test]
fn test_two_line_scroll_wheel_down_scrolls_viewport() {
    let mut app = make_two_line_app(10, 80);
    app.compact_scroll_offset = 3;
    app.selected_index = Some(3);

    let mouse = make_mouse_event(MouseEventKind::ScrollDown, 0, 10);
    let action = app.handle_mouse_event(mouse);
    assert_eq!(action, Action::None);
    assert_eq!(
        app.compact_scroll_offset, 4,
        "should scroll viewport right"
    );
    // Selection should not change on scroll wheel
    assert_eq!(app.selected_index, Some(3), "selection unchanged");
}

#[test]
fn test_two_line_scroll_wheel_up_scrolls_viewport() {
    let mut app = make_two_line_app(10, 80);
    app.compact_scroll_offset = 3;
    app.selected_index = Some(5);

    let mouse = make_mouse_event(MouseEventKind::ScrollUp, 0, 10);
    let action = app.handle_mouse_event(mouse);
    assert_eq!(action, Action::None);
    assert_eq!(
        app.compact_scroll_offset, 2,
        "should scroll viewport left"
    );
    // Selection should not change on scroll wheel
    assert_eq!(app.selected_index, Some(5), "selection unchanged");
}

#[test]
fn test_two_line_click_outside_chips_clears_selection() {
    let mut app = make_two_line_app(3, 80);
    app.selected_index = Some(1);

    // Click on row 1 (not session chips row)
    let mouse = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 1, 10);
    let action = app.handle_mouse_event(mouse);
    assert_eq!(action, Action::None);
    // Selection should remain (only row 0 is interactive)
    assert_eq!(app.selected_index, Some(1));
}

#[test]
fn test_two_line_double_click_fires_hook() {
    let mut app = make_two_line_app(3, 80);
    app.activate_hook = Some("echo test".to_string());

    // First click
    let mouse1 = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 0, 10);
    app.handle_mouse_event(mouse1);
    assert_eq!(app.selected_index, Some(0));

    // Second click (double-click)
    let mouse2 = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 0, 10);
    app.handle_mouse_event(mouse2);
    assert!(app.status_message.is_some());
    let (msg, _) = app.status_message.as_ref().expect("msg");
    assert_eq!(msg, "Hook executed");
}

#[test]
fn test_keyboard_left_arrow_focuses_new_leftmost() {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    let mut app = make_two_line_app(10, 80);
    app.compact_scroll_offset = 5;
    app.selected_index = Some(5);

    let key = KeyEvent {
        code: KeyCode::Left,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };
    let action = handle_key_event(&mut app, key);
    assert_eq!(action, Action::None);
    assert_eq!(app.compact_scroll_offset, 4, "should scroll left");
    assert_eq!(
        app.selected_index,
        Some(4),
        "should focus new leftmost chip"
    );
}

#[test]
fn test_keyboard_right_arrow_focuses_new_rightmost() {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    let mut app = make_two_line_app(10, 80);
    app.compact_scroll_offset = 0;
    app.selected_index = Some(0);

    let key = KeyEvent {
        code: KeyCode::Right,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };
    let action = handle_key_event(&mut app, key);
    assert_eq!(action, Action::None);
    assert_eq!(app.compact_scroll_offset, 1, "should scroll right");
    // Rightmost: offset(1) + max_visible(3) - 1 = 3
    assert_eq!(
        app.selected_index,
        Some(3),
        "should focus new rightmost chip"
    );
}

// --- Render integration tests (test visual output, not click logic) ---

use crate::tui::test_utils::{find_row_with_text, render_dashboard_to_buffer};
use ratatui::style::Color;

#[test]
fn test_click_selects_and_renders_highlight() {
    let mut app = make_clickable_app(5);
    app.selected_index = Some(2);
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
    assert!(found_highlight, "Selected session should have highlight");
}

#[test]
fn test_no_selection_renders_no_highlight() {
    let mut app = make_app_with_sessions(3);
    app.selected_index = None;
    let buffer = render_dashboard_to_buffer(&mut app, 80, 30);
    for row in 0..buffer.area().height {
        for col in 0..buffer.area().width {
            if let Some(cell) = buffer.cell((col, row)) {
                assert!(
                    cell.bg != Color::DarkGray,
                    "No row should have highlight when selected_index is None, but row {} does",
                    row
                );
            }
        }
    }
}
