use super::*;
use crate::tui::test_utils::{
    assert_text_bg_in_row, assert_text_fg_in_row, find_row_with_text, make_inactive_session,
    make_session as make_test_session_with_dir, render_dashboard_to_buffer,
    render_session_list_to_buffer, row_contains, row_text,
};

// Buffer Content Tests (8 tests - verify existing behavior)

#[test]
fn test_dashboard_buffer_contains_header_text() {
    use crate::tui::app::App;
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));
    let buffer = render_dashboard_to_buffer(&mut app, 80, 24);
    assert!(
        find_row_with_text(&buffer, "Agent Console Dashboard").is_some(),
        "Buffer should contain header text"
    );
}

#[test]
fn test_dashboard_buffer_contains_footer_keybindings() {
    use crate::tui::app::App;
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));
    let buffer = render_dashboard_to_buffer(&mut app, 80, 24);
    let last_row = buffer.area().height - 1;
    assert!(
        row_contains(&buffer, last_row, "[q] Quit"),
        "Footer should contain keybindings"
    );
}

#[test]
fn test_dashboard_buffer_contains_session_border() {
    use crate::tui::app::App;
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));
    let buffer = render_dashboard_to_buffer(&mut app, 80, 24);
    assert!(
        find_row_with_text(&buffer, "Sessions").is_some(),
        "Buffer should contain 'Sessions' border title"
    );
}

#[test]
fn test_dashboard_buffer_shows_session_names() {
    use crate::tui::app::App;
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));
    app.sessions.push(make_test_session_with_dir(
        "test-session-id",
        Status::Working,
        Some(PathBuf::from("/tmp")),
    ));
    app.init_selection();
    let buffer = render_dashboard_to_buffer(&mut app, 80, 24);
    assert!(
        find_row_with_text(&buffer, "test-session-id").is_some(),
        "Buffer should contain session ID"
    );
}

#[test]
fn test_dashboard_empty_renders_without_session_text() {
    use crate::tui::app::App;
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));
    let buffer = render_dashboard_to_buffer(&mut app, 80, 24);
    for row in 0..buffer.area().height {
        let text = row_text(&buffer, row);
        assert!(
            !text.contains("session-"),
            "Empty dashboard should not contain session IDs"
        );
    }
}

#[test]
fn test_dashboard_selected_session_has_highlight() {
    use crate::tui::app::App;
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));
    app.sessions.push(make_test_session_with_dir(
        "highlighted",
        Status::Working,
        Some(PathBuf::from("/tmp")),
    ));
    app.init_selection();
    let buffer = render_dashboard_to_buffer(&mut app, 80, 24);
    let session_row = find_row_with_text(&buffer, "highlighted").expect("should find session row");
    assert_text_bg_in_row(&buffer, session_row, "highlighted", Color::DarkGray);
}

#[test]
fn test_dashboard_selected_session_has_arrow_symbol() {
    use crate::tui::app::App;
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));
    app.sessions.push(make_test_session_with_dir(
        "with-arrow",
        Status::Working,
        Some(PathBuf::from("/tmp")),
    ));
    app.init_selection();
    let buffer = render_dashboard_to_buffer(&mut app, 80, 24);
    let session_row = find_row_with_text(&buffer, "with-arrow").expect("should find session");
    let row_string = row_text(&buffer, session_row);
    assert!(
        row_string.contains('▶'),
        "Selected session should have arrow highlight symbol: '{}'",
        row_string
    );
}

#[test]
fn test_narrow_mode_shows_only_symbol_and_name() {
    let sessions = vec![make_test_session_with_dir(
        "narrow-test",
        Status::Working,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, Some(0), 35, 10);
    let row = find_row_with_text(&buffer, "narrow-test").expect("should find session");
    let row_string = row_text(&buffer, row);
    assert!(
        row_string.contains("narrow-test"),
        "Narrow mode should show session ID"
    );
    assert!(
        !row_string.contains("working"),
        "Narrow mode should not show status column: '{}'",
        row_string
    );
}

// Status Color Tests (6 tests)

#[test]
fn test_working_status_renders_green() {
    let sessions = vec![make_test_session_with_dir(
        "test-sess",
        Status::Working,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, None, 80, 10);
    let row = find_row_with_text(&buffer, "test-sess").expect("should find session");
    assert_text_fg_in_row(&buffer, row, "working", Color::Green);
}

#[test]
fn test_attention_status_renders_yellow() {
    let sessions = vec![make_test_session_with_dir(
        "test-sess",
        Status::Attention,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, None, 80, 10);
    let row = find_row_with_text(&buffer, "test-sess").expect("should find session");
    assert_text_fg_in_row(&buffer, row, "attention", Color::Yellow);
}

#[test]
fn test_question_status_renders_blue() {
    let sessions = vec![make_test_session_with_dir(
        "test-sess",
        Status::Question,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, None, 80, 10);
    let row = find_row_with_text(&buffer, "test-sess").expect("should find session");
    assert_text_fg_in_row(&buffer, row, "question", Color::Blue);
}

#[test]
fn test_closed_status_renders_gray() {
    let sessions = vec![make_test_session_with_dir(
        "test-sess",
        Status::Closed,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, None, 80, 10);
    let row = find_row_with_text(&buffer, "test-sess").expect("should find session");
    assert_text_fg_in_row(&buffer, row, "closed", Color::DarkGray);
}

#[test]
fn test_inactive_session_renders_dark_gray() {
    let session = make_inactive_session("test-sess", INACTIVE_SESSION_THRESHOLD.as_secs() + 100);
    let sessions = vec![session];
    let buffer = render_session_list_to_buffer(&sessions, None, 80, 10);
    let row = find_row_with_text(&buffer, "test-sess").expect("should find session");
    assert_text_fg_in_row(&buffer, row, "inactive", Color::DarkGray);
}

#[test]
fn test_inactive_session_highlighted_uses_black_text() {
    let session =
        make_inactive_session("test-inactive", INACTIVE_SESSION_THRESHOLD.as_secs() + 100);
    let sessions = vec![session];
    let buffer = render_session_list_to_buffer(&sessions, Some(0), 80, 10);
    let row = find_row_with_text(&buffer, "test-inactive").expect("should find session");
    assert_text_fg_in_row(&buffer, row, "test-inactive", Color::Black);
    assert_text_bg_in_row(&buffer, row, "test-inactive", Color::DarkGray);
}

#[test]
fn test_error_working_dir_renders_red() {
    let sessions = vec![make_test_session_with_dir(
        "error-test",
        Status::Working,
        None,
    )];
    // Use width 100 to ensure all columns fit (fixed width is 84 + at least 1 for directory = 85 minimum)
    let buffer = render_session_list_to_buffer(&sessions, None, 100, 10);
    let row = find_row_with_text(&buffer, "error-test").expect("should find session");
    assert_text_fg_in_row(&buffer, row, "<error>", Color::Red);
}

// Responsive Layout Tests (4 tests)

#[test]
fn test_standard_mode_shows_all_columns() {
    let sessions = vec![make_test_session_with_dir(
        "standard",
        Status::Working,
        Some(PathBuf::from("/home/user/project")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, None, 100, 10);
    let row = find_row_with_text(&buffer, "standard").expect("should find session");
    let row_string = row_text(&buffer, row);
    assert!(row_string.contains("project"), "Should show directory");
    assert!(row_string.contains("standard"), "Should show session ID");
    assert!(row_string.contains("working"), "Should show status");
    assert!(
        row_string.contains('s') || row_string.contains('m'),
        "Should show elapsed"
    );
}

#[test]
fn test_wide_mode_shows_wider_directory() {
    let long_dir = "very-long-project-directory-name";
    let sessions = vec![make_test_session_with_dir(
        "wide",
        Status::Working,
        Some(PathBuf::from(format!("/home/user/{}", long_dir))),
    )];
    let buffer_standard = render_session_list_to_buffer(&sessions, None, 60, 10);
    let buffer_wide = render_session_list_to_buffer(&sessions, None, 100, 10);

    let row_standard = find_row_with_text(&buffer_standard, "wide").expect("should find session");
    let row_wide = find_row_with_text(&buffer_wide, "wide").expect("should find session");

    let text_standard = row_text(&buffer_standard, row_standard);
    let text_wide = row_text(&buffer_wide, row_wide);

    let dir_chars_standard = text_standard
        .matches(|c: char| c.is_alphanumeric() && long_dir.contains(c))
        .count();
    let dir_chars_wide = text_wide
        .matches(|c: char| c.is_alphanumeric() && long_dir.contains(c))
        .count();

    assert!(
        dir_chars_wide >= dir_chars_standard,
        "Wide mode should show at least as much directory content as standard mode"
    );
}

#[test]
fn test_header_row_is_cyan_bold() {
    let sessions = vec![make_test_session_with_dir(
        "test",
        Status::Working,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, Some(0), 60, 10);
    let header_row = find_row_with_text(&buffer, "Directory").expect("should find header");
    let dir_col = row_text(&buffer, header_row)
        .find("Directory")
        .expect("Directory not found") as u16;
    let cell = buffer
        .cell((dir_col, header_row))
        .expect("cell should exist");
    assert_eq!(cell.fg, Color::Cyan, "Header should be cyan");
    assert!(
        cell.modifier.contains(ratatui::style::Modifier::BOLD),
        "Header should be bold"
    );
}

#[test]
fn test_header_row_absent_in_narrow_mode() {
    let sessions = vec![make_test_session_with_dir(
        "narrow",
        Status::Working,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, Some(0), 35, 10);
    assert!(
        find_row_with_text(&buffer, "Directory").is_none(),
        "Narrow mode should not have header row"
    );
    assert!(
        find_row_with_text(&buffer, "Session ID").is_none(),
        "Narrow mode should not have Session ID header"
    );
}

// --- Debug Ruler Tests (acd-2rk) ---

#[test]
fn test_format_ruler_line_standard_width() {
    let line = format_ruler_line(100);
    let spans: Vec<&str> = line.spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(spans.len(), 6);
    assert!(spans[1].contains("dir:"), "should show dir width label");
    assert!(
        spans[2].contains("stat:14"),
        "should show status width label"
    );
    assert!(
        spans[3].contains("prio:12"),
        "should show priority width label"
    );
    assert!(
        spans[4].contains("time:16"),
        "should show elapsed width label"
    );
    assert!(spans[5].contains("id:40"), "should show id width label");
}

#[test]
fn test_format_ruler_line_narrow_empty() {
    let line = format_ruler_line(30);
    assert!(line.spans.is_empty(), "narrow mode should have no ruler");
}

#[test]
fn test_debug_ruler_disabled_by_default() {
    assert!(
        !debug_ruler_enabled()
            || std::env::var("AGENT_CONSOLE_DASHBOARD_DEBUG")
                .ok()
                .as_deref()
                == Some("1")
    );
}

// --- Story acd-88r: Closed sessions should use same visual style as inactive ---

#[test]
fn test_closed_session_renders_dimmed() {
    let sessions = vec![make_test_session_with_dir(
        "closed-sess",
        Status::Closed,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, None, 80, 10);
    let row = find_row_with_text(&buffer, "closed-sess").expect("should find session");
    assert_text_fg_in_row(&buffer, row, "closed", Color::DarkGray);
}

#[test]
fn test_closed_session_highlighted_uses_black_text() {
    let sessions = vec![make_test_session_with_dir(
        "closed-highlighted",
        Status::Closed,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, Some(0), 80, 10);
    let row = find_row_with_text(&buffer, "closed-highlighted").expect("should find session");
    assert_text_fg_in_row(&buffer, row, "closed-highlighted", Color::Black);
    assert_text_bg_in_row(&buffer, row, "closed-highlighted", Color::DarkGray);
}

#[test]
fn test_closed_session_not_highlighted_uses_dark_gray_text() {
    let sessions = vec![make_test_session_with_dir(
        "closed-not-highlighted",
        Status::Closed,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, None, 80, 10);
    let row = find_row_with_text(&buffer, "closed-not-highlighted").expect("should find session");
    assert_text_fg_in_row(&buffer, row, "closed-not-highlighted", Color::DarkGray);
}

#[test]
fn test_working_session_not_dimmed() {
    let sessions = vec![make_test_session_with_dir(
        "working-sess",
        Status::Working,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, None, 80, 10);
    let row = find_row_with_text(&buffer, "working-sess").expect("should find session");
    let row_str = row_text(&buffer, row);
    let col = row_str
        .find("working-sess")
        .expect("should find session id text");
    let cell = buffer.cell((col as u16, row)).expect("cell should exist");
    assert_ne!(
        cell.fg,
        Color::DarkGray,
        "working session should not be dimmed"
    );
}

// --- Story acd-kmvh: All columns should have consistent color for inactive/closed sessions ---

#[test]
fn test_inactive_session_all_columns_same_color() {
    let session =
        make_inactive_session("inactive-test", INACTIVE_SESSION_THRESHOLD.as_secs() + 100);
    let sessions = vec![session];
    let buffer = render_session_list_to_buffer(&sessions, None, 100, 10);
    let row = find_row_with_text(&buffer, "inactive-test").expect("should find session");

    // All text in the row (directory, status, priority, elapsed, session_id) should be DarkGray
    assert_text_fg_in_row(&buffer, row, "test", Color::DarkGray); // directory
    assert_text_fg_in_row(&buffer, row, "inactive", Color::DarkGray); // status
    assert_text_fg_in_row(&buffer, row, "0", Color::DarkGray); // priority
    assert_text_fg_in_row(&buffer, row, "inactive-test", Color::DarkGray); // session_id
}

#[test]
fn test_closed_session_all_columns_same_color() {
    let sessions = vec![make_test_session_with_dir(
        "closed-test",
        Status::Closed,
        Some(PathBuf::from("/tmp/testdir")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, None, 100, 10);
    let row = find_row_with_text(&buffer, "closed-test").expect("should find session");

    // All text in the row (directory, status, priority, elapsed, session_id) should be DarkGray
    assert_text_fg_in_row(&buffer, row, "testdir", Color::DarkGray); // directory
    assert_text_fg_in_row(&buffer, row, "closed", Color::DarkGray); // status
    assert_text_fg_in_row(&buffer, row, "0", Color::DarkGray); // priority
    assert_text_fg_in_row(&buffer, row, "closed-test", Color::DarkGray); // session_id
}

#[test]
fn test_inactive_session_highlighted_all_columns_black() {
    let session = make_inactive_session("inactive-hl", INACTIVE_SESSION_THRESHOLD.as_secs() + 100);
    let sessions = vec![session];
    let buffer = render_session_list_to_buffer(&sessions, Some(0), 100, 10);
    let row = find_row_with_text(&buffer, "inactive-hl").expect("should find session");

    // When highlighted, all text should be Black (for readability against DarkGray background)
    assert_text_fg_in_row(&buffer, row, "test", Color::Black); // directory
    assert_text_fg_in_row(&buffer, row, "inactive", Color::Black); // status
    assert_text_fg_in_row(&buffer, row, "0", Color::Black); // priority
    assert_text_fg_in_row(&buffer, row, "inactive-hl", Color::Black); // session_id
}

#[test]
fn test_closed_session_highlighted_all_columns_black() {
    let sessions = vec![make_test_session_with_dir(
        "closed-hl",
        Status::Closed,
        Some(PathBuf::from("/tmp/closeddir")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, Some(0), 100, 10);
    let row = find_row_with_text(&buffer, "closed-hl").expect("should find session");

    // When highlighted, all text should be Black (for readability against DarkGray background)
    assert_text_fg_in_row(&buffer, row, "closeddir", Color::Black); // directory
    assert_text_fg_in_row(&buffer, row, "closed", Color::Black); // status
    assert_text_fg_in_row(&buffer, row, "0", Color::Black); // priority
    assert_text_fg_in_row(&buffer, row, "closed-hl", Color::Black); // session_id
}
