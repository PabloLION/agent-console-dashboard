use super::*;
use std::time::Instant;


#[test]
fn test_status_color_mapping() {
    assert_eq!(status_color_for(Status::Working), Color::Green);
    assert_eq!(status_color_for(Status::Attention), Color::Yellow);
    assert_eq!(status_color_for(Status::Question), Color::Magenta);
    assert_eq!(status_color_for(Status::Closed), Color::DarkGray);
}

#[test]
fn test_history_shows_per_state_duration_not_ago() {
    let mut session = make_session("duration-test");
    let now = Instant::now();

    // Add a transition that happened 60 seconds ago and lasted 30 seconds
    session.history.push(StateTransition {
        timestamp: now - Duration::from_secs(60),
        from: Status::Working,
        to: Status::Attention,
        duration: Duration::from_secs(30),
    });

    let lines = build_detail_lines(&session, 60, 0, now, true);
    let text: String = lines
        .iter()
        .flat_map(|line| line.spans.iter().map(|span| span.content.as_ref()))
        .collect();

    // Should NOT contain "ago" - that's the old behavior
    assert!(
        !text.contains("ago"),
        "History should not show 'ago' format, got: '{}'",
        text
    );

    // Should contain the current elapsed time (60s since the transition)
    assert!(
        text.contains("1m"),
        "History should show current duration since transition, got: '{}'",
        text
    );
}

#[test]
fn test_history_most_recent_shows_dynamic_duration() {
    let mut session = make_session("dynamic-test");
    let now = Instant::now();

    // Most recent transition - happened 45 seconds ago
    session.history.push(StateTransition {
        timestamp: now - Duration::from_secs(45),
        from: Status::Working,
        to: Status::Attention,
        duration: Duration::from_secs(10), // This duration is ignored for most recent
    });

    let lines = build_detail_lines(&session, 60, 0, now, true);
    let text: String = lines
        .iter()
        .flat_map(|line| line.spans.iter().map(|span| span.content.as_ref()))
        .collect();

    // Most recent transition should show 45s (time since transition)
    assert!(
        text.contains("45s"),
        "Most recent transition should show current elapsed time (45s), got: '{}'",
        text
    );
}

#[test]
fn test_history_older_transitions_use_stored_duration() {
    let mut session = make_session("stored-duration-test");
    let now = Instant::now();

    // Older transition (not most recent)
    session.history.push(StateTransition {
        timestamp: now - Duration::from_secs(200),
        from: Status::Working,
        to: Status::Attention,
        duration: Duration::from_secs(120), // 2 minutes
    });

    // Most recent transition
    session.history.push(StateTransition {
        timestamp: now - Duration::from_secs(50),
        from: Status::Attention,
        to: Status::Working,
        duration: Duration::from_secs(150), // This is ignored for most recent
    });

    let lines = build_detail_lines(&session, 80, 0, now, true);
    let text: String = lines
        .iter()
        .flat_map(|line| line.spans.iter().map(|span| span.content.as_ref()))
        .collect();

    // Should show 2m for the older transition (stored duration)
    assert!(
        text.contains("2m"),
        "Older transition should show stored duration (2m), got: '{}'",
        text
    );

    // Should show 50s for the most recent transition
    assert!(
        text.contains("50s"),
        "Most recent transition should show current elapsed (50s), got: '{}'",
        text
    );
}

#[test]
fn test_history_multiple_transitions_show_correct_durations() {
    let mut session = make_session("multi-duration-test");
    let now = Instant::now();

    // Build a history with multiple transitions
    // Oldest: working→attention (lasted 5 minutes)
    session.history.push(StateTransition {
        timestamp: now - Duration::from_secs(600),
        from: Status::Working,
        to: Status::Attention,
        duration: Duration::from_secs(300), // 5m
    });

    // Middle: attention→question (lasted 2 minutes)
    session.history.push(StateTransition {
        timestamp: now - Duration::from_secs(300),
        from: Status::Attention,
        to: Status::Question,
        duration: Duration::from_secs(120), // 2m
    });

    // Most recent: question→working (30 seconds ago, still ongoing)
    session.history.push(StateTransition {
        timestamp: now - Duration::from_secs(30),
        from: Status::Question,
        to: Status::Working,
        duration: Duration::from_secs(0), // Ignored for most recent
    });

    let lines = build_detail_lines(&session, 80, 0, now, true);

    // Verify the content contains expected durations
    let text: String = lines
        .iter()
        .flat_map(|line| line.spans.iter().map(|span| span.content.as_ref()))
        .collect();

    // Most recent should show 30s
    assert!(
        text.contains("30s"),
        "Most recent should show 30s, got: '{}'",
        text
    );

    // Should contain 2m for middle transition
    assert!(
        text.contains("2m"),
        "Middle transition should show 2m duration, got: '{}'",
        text
    );

    // Should contain 5m for oldest transition
    assert!(
        text.contains("5m"),
        "Oldest transition should show 5m duration, got: '{}'",
        text
    );
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
