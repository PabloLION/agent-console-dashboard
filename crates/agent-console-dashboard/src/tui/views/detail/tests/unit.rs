use super::*;
use std::time::Instant;

#[test]
fn test_format_transition_time_seconds() {
    let now = Instant::now();
    let ts = now - Duration::from_secs(30);
    let result = format_transition_time(ts, now);
    assert_eq!(result, "30s ago");
}

#[test]
fn test_format_transition_time_minutes() {
    let now = Instant::now();
    let ts = now - Duration::from_secs(150);
    let result = format_transition_time(ts, now);
    assert_eq!(result, "2m 30s ago");
}

#[test]
fn test_format_transition_time_hours() {
    let now = Instant::now();
    let ts = now - Duration::from_secs(3661);
    let result = format_transition_time(ts, now);
    assert_eq!(result, "1h 1m 1s ago");
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
    session.working_dir = Some(PathBuf::from(
        "/very/deeply/nested/path/to/some/project/directory/that/is/quite/long",
    ));
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
    assert!(
        lines_without.len() < lines_with.len(),
        "inline mode should have fewer lines than modal"
    );
}

// --- Story 5 (acd-4sq): Detail panel "unknown" → "<error>" tests ---

#[test]
fn test_build_detail_lines_unknown_working_dir_shows_error() {
    let mut session = Session::new("test-unknown-dir".to_string(), AgentType::ClaudeCode, None);
    session.status = Status::Working;
    let lines = build_detail_lines(&session, 60, 0, Instant::now(), true);

    let dir_line = &lines[1];
    let full_text: String = dir_line.spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(
        full_text.contains("<error>"),
        "Dir line should contain '<error>' for unknown path, got: '{}'",
        full_text
    );

    let wd_span = &dir_line.spans[1];
    assert_eq!(
        wd_span.style.fg,
        Some(Color::Red),
        "Unknown working dir should be styled with red"
    );
}

#[test]
fn test_build_detail_lines_normal_working_dir() {
    let session = make_session("test-normal-dir");
    let lines = build_detail_lines(&session, 60, 0, Instant::now(), true);

    let dir_line = &lines[1];
    let full_text: String = dir_line.spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(
        !full_text.contains("<error>"),
        "Dir line should not contain '<error>' for normal path, got: '{}'",
        full_text
    );
    assert!(
        full_text.contains("project-a"),
        "Dir line should contain path component, got: '{}'",
        full_text
    );

    let wd_span = &dir_line.spans[1];
    assert_ne!(
        wd_span.style.fg,
        Some(Color::Red),
        "Normal working dir should not be red"
    );
}

#[test]
fn test_render_detail_unknown_working_dir_no_panic() {
    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    let session = Session::new(
        "test-unknown-render".to_string(),
        AgentType::ClaudeCode,
        None,
    );
    terminal
        .draw(|frame| {
            render_detail(frame, &session, frame.area(), 0, Instant::now());
        })
        .expect("draw should not fail with unknown working_dir");
}

#[test]
fn test_render_inline_detail_unknown_working_dir_no_panic() {
    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    let session = Session::new(
        "test-unknown-inline".to_string(),
        AgentType::ClaudeCode,
        None,
    );
    terminal
        .draw(|frame| {
            render_inline_detail(frame, &session, frame.area(), 0, Instant::now());
        })
        .expect("draw should not fail with unknown working_dir");
}
