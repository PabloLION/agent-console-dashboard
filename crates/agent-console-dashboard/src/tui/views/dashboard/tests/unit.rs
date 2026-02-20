use super::*;

// --- status_symbol tests ---

#[test]
fn test_status_symbol_working() {
    assert_eq!(status_symbol(Status::Working), "*");
}

#[test]
fn test_status_symbol_attention() {
    assert_eq!(status_symbol(Status::Attention), "!");
}

#[test]
fn test_status_symbol_question() {
    assert_eq!(status_symbol(Status::Question), "?");
}

#[test]
fn test_status_symbol_closed() {
    assert_eq!(status_symbol(Status::Closed), "x");
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
    assert_eq!(format_elapsed_seconds(3661), "1h 1m 1s");
}

#[test]
fn test_format_elapsed_seconds_exact_hour() {
    assert_eq!(format_elapsed_seconds(3600), "1h 0m 0s");
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
    let line = format_session_line(&session, 30, "project", false);
    // Should have exactly 2 spans (symbol + session ID)
    assert_eq!(line.spans.len(), 2);
}

#[test]
fn test_format_session_line_standard() {
    let session = make_session("my-session", Status::Attention);
    let line = format_session_line(&session, 60, "project", false);
    // Should have 5 spans (workdir, status, priority, elapsed, session ID) — highlight handled by List widget
    assert_eq!(line.spans.len(), 5);
}

#[test]
fn test_format_session_line_wide() {
    let session = make_session("my-session", Status::Question);
    let line = format_session_line(&session, 100, "project", false);
    // Wide has same 5 spans as standard (workdir, status, priority, elapsed, session ID)
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
    let standard_line = format_session_line(&session, 60, long_name, false);
    let wide_line = format_session_line(&session, 100, long_name, false);

    // work_dir span is index 0 in both modes (highlight handled by List widget)
    let standard_dir = &standard_line.spans[0];
    let wide_dir = &wide_line.spans[0];

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
    let line = format_session_line(&session, 80, "project", false);

    // Session ID span is index 4 (directory, status, priority, elapsed, session_id)
    let name_span = &line.spans[4];
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
        let _ = format_session_line(&session, 20, "project", false);
        let _ = format_session_line(&session, 50, "project", false);
        let _ = format_session_line(&session, 120, "project", false);
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
