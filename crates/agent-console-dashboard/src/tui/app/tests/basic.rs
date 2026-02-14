use super::*;

#[test]
fn test_app_new() {
    let app = App::new(PathBuf::from("/tmp/test.sock"));
    assert!(!app.should_quit);
    assert_eq!(app.socket_path, PathBuf::from("/tmp/test.sock"));
    assert_eq!(app.tick_count, 0);
    assert!(app.sessions.is_empty());
    assert!(app.selected_index.is_none());
    assert_eq!(app.view, View::Dashboard);
    assert_eq!(app.layout_preset, 1);
    assert!(app.usage.is_none());
}

#[test]
fn test_app_default_state() {
    let app = App::new(PathBuf::from("/tmp/agent-console.sock"));
    assert!(!app.should_quit);
    assert_eq!(app.tick_count, 0);
    assert!(app.sessions.is_empty());
    assert!(app.selected_index.is_none());
    assert_eq!(app.view, View::Dashboard);
}

#[test]
fn test_app_tick_increment() {
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));
    assert_eq!(app.tick_count, 0);
    app.tick_count += 1;
    assert_eq!(app.tick_count, 1);
    app.tick_count += 1;
    assert_eq!(app.tick_count, 2);
}

#[test]
fn test_app_should_quit_toggle() {
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));
    assert!(!app.should_quit);
    app.should_quit = true;
    assert!(app.should_quit);
}

#[test]
fn test_app_socket_path() {
    let app = App::new(PathBuf::from("/custom/path.sock"));
    assert_eq!(app.socket_path, PathBuf::from("/custom/path.sock"));
}

#[test]
fn test_app_debug_format() {
    let app = App::new(PathBuf::from("/tmp/debug.sock"));
    let debug = format!("{:?}", app);
    assert!(debug.contains("should_quit"));
    assert!(debug.contains("socket_path"));
    assert!(debug.contains("tick_count"));
    assert!(debug.contains("sessions"));
    assert!(debug.contains("selected_index"));
    assert!(debug.contains("view"));
    assert!(debug.contains("layout_preset"));
}

// --- init_selection tests ---

#[test]
fn test_init_selection_empty() {
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));
    app.init_selection();
    assert_eq!(app.selected_index, None);
}

#[test]
fn test_init_selection_with_sessions() {
    let app = make_app_with_sessions(3);
    assert_eq!(app.selected_index, Some(0));
}

// --- select_next tests ---

#[test]
fn test_select_next_moves_down() {
    let mut app = make_app_with_sessions(3);
    app.select_next();
    assert_eq!(app.selected_index, Some(1));
    app.select_next();
    assert_eq!(app.selected_index, Some(2));
}

#[test]
fn test_select_next_clamps_at_end() {
    let mut app = make_app_with_sessions(3);
    app.selected_index = Some(2);
    app.select_next();
    assert_eq!(app.selected_index, Some(2));
}

#[test]
fn test_select_next_empty_sessions() {
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));
    app.select_next();
    assert_eq!(app.selected_index, None);
}

// --- select_previous tests ---

#[test]
fn test_select_previous_moves_up() {
    let mut app = make_app_with_sessions(3);
    app.selected_index = Some(2);
    app.select_previous();
    assert_eq!(app.selected_index, Some(1));
}

#[test]
fn test_select_previous_clamps_at_zero() {
    let mut app = make_app_with_sessions(3);
    app.select_previous();
    assert_eq!(app.selected_index, Some(0));
}

#[test]
fn test_select_previous_empty_sessions() {
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));
    app.select_previous();
    assert_eq!(app.selected_index, None);
}

// --- selected_session tests ---

#[test]
fn test_selected_session_returns_correct() {
    let app = make_app_with_sessions(3);
    let session = app
        .selected_session()
        .expect("should have selected session");
    assert_eq!(session.session_id, "session-0");
}

#[test]
fn test_selected_session_none_when_empty() {
    let app = App::new(PathBuf::from("/tmp/test.sock"));
    assert!(app.selected_session().is_none());
}

#[test]
fn test_selected_session_after_navigation() {
    let mut app = make_app_with_sessions(5);
    app.select_next();
    app.select_next();
    let session = app
        .selected_session()
        .expect("should have selected session");
    assert_eq!(session.session_id, "session-2");
}

// --- integration: multiple nav steps ---

#[test]
fn test_navigation_sequence() {
    let mut app = make_app_with_sessions(4);
    // Down to end
    app.select_next();
    app.select_next();
    app.select_next();
    assert_eq!(app.selected_index, Some(3));
    // Try going past end
    app.select_next();
    assert_eq!(app.selected_index, Some(3));
    // Back up to start
    app.select_previous();
    app.select_previous();
    app.select_previous();
    assert_eq!(app.selected_index, Some(0));
    // Try going past start
    app.select_previous();
    assert_eq!(app.selected_index, Some(0));
}

#[test]
fn test_single_session_navigation() {
    let mut app = make_app_with_sessions(1);
    assert_eq!(app.selected_index, Some(0));
    app.select_next();
    assert_eq!(app.selected_index, Some(0));
    app.select_previous();
    assert_eq!(app.selected_index, Some(0));
}

// --- View state tests ---

#[test]
fn test_open_detail_sets_view() {
    let mut app = make_app_with_sessions(3);
    app.open_detail(1);
    // open_detail is now deprecated (detail is always visible)
    // View should still be Dashboard
    assert_eq!(app.view, View::Dashboard);
}

#[test]
fn test_open_detail_out_of_bounds_no_change() {
    let mut app = make_app_with_sessions(3);
    app.open_detail(5);
    assert_eq!(app.view, View::Dashboard);
}

#[test]
fn test_close_detail_returns_to_dashboard() {
    let mut app = make_app_with_sessions(3);
    app.selected_index = Some(0);
    app.close_detail();
    // close_detail now clears selection (defocus)
    assert_eq!(app.selected_index, None);
    assert_eq!(app.history_scroll, 0);
}

#[test]
fn test_scroll_history_down() {
    let mut app = make_app_with_sessions(1);
    // Add enough history entries
    for _ in 0..10 {
        app.sessions[0].history.push(crate::StateTransition {
            timestamp: std::time::Instant::now(),
            from: crate::Status::Working,
            to: crate::Status::Attention,
            duration: std::time::Duration::from_secs(1),
        });
    }
    app.selected_index = Some(0);
    app.scroll_history_down();
    // History scroll is now tracked separately from View
    assert_eq!(app.history_scroll, 1);
    assert_eq!(app.view, View::Dashboard);
}

#[test]
fn test_scroll_history_up() {
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
    app.history_scroll = 3;
    app.scroll_history_up();
    // History scroll is now a field on App
    assert_eq!(app.history_scroll, 2);
    assert_eq!(app.view, View::Dashboard);
}

#[test]
fn test_scroll_history_up_clamps_at_zero() {
    let mut app = make_app_with_sessions(1);
    app.selected_index = Some(0);
    app.scroll_history_up();
    assert_eq!(app.history_scroll, 0);
    assert_eq!(app.view, View::Dashboard);
}

#[test]
fn test_layout_preset_default() {
    let app = App::new(PathBuf::from("/tmp/test.sock"));
    assert_eq!(app.layout_preset, 1);
}

// --- App usage field tests ---

#[test]
fn test_app_usage_starts_none() {
    let app = App::new(PathBuf::from("/tmp/test.sock"));
    assert!(app.usage.is_none());
}

// --- Session sorting tests ---

#[test]
fn test_session_sort_by_status_group() {
    use crate::SessionSnapshot;
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));

    // Create sessions with different statuses
    let attention = SessionSnapshot {
        session_id: "attention-1".to_string(),
        agent_type: "claudecode".to_string(),
        status: "attention".to_string(),
        working_dir: None,
        elapsed_seconds: 10,
        idle_seconds: 5,
        history: vec![],
        closed: false,
        priority: 0,
    };

    let working = SessionSnapshot {
        session_id: "working-1".to_string(),
        agent_type: "claudecode".to_string(),
        status: "working".to_string(),
        working_dir: None,
        elapsed_seconds: 10,
        idle_seconds: 5,
        history: vec![],
        closed: false,
        priority: 0,
    };

    let closed = SessionSnapshot {
        session_id: "closed-1".to_string(),
        agent_type: "claudecode".to_string(),
        status: "closed".to_string(),
        working_dir: None,
        elapsed_seconds: 10,
        idle_seconds: 5,
        history: vec![],
        closed: true,
        priority: 0,
    };

    // Apply in reverse order: closed, working, attention
    app.apply_update(&closed);
    app.apply_update(&working);
    app.apply_update(&attention);

    // After sorting, attention should be first, working second, closed last
    assert_eq!(app.sessions[0].session_id, "attention-1");
    assert_eq!(app.sessions[1].session_id, "working-1");
    assert_eq!(app.sessions[2].session_id, "closed-1");
}

#[test]
fn test_session_sort_by_priority() {
    use crate::SessionSnapshot;
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));

    // Create sessions with same status but different priorities
    let low_priority = SessionSnapshot {
        session_id: "low".to_string(),
        agent_type: "claudecode".to_string(),
        status: "working".to_string(),
        working_dir: None,
        elapsed_seconds: 10,
        idle_seconds: 5,
        history: vec![],
        closed: false,
        priority: 1,
    };

    let high_priority = SessionSnapshot {
        session_id: "high".to_string(),
        agent_type: "claudecode".to_string(),
        status: "working".to_string(),
        working_dir: None,
        elapsed_seconds: 10,
        idle_seconds: 5,
        history: vec![],
        closed: false,
        priority: 10,
    };

    // Apply in wrong order
    app.apply_update(&low_priority);
    app.apply_update(&high_priority);

    // After sorting, high priority should be first
    assert_eq!(app.sessions[0].session_id, "high");
    assert_eq!(app.sessions[0].priority, 10);
    assert_eq!(app.sessions[1].session_id, "low");
    assert_eq!(app.sessions[1].priority, 1);
}

#[test]
fn test_session_sort_by_elapsed_time() {
    use crate::SessionSnapshot;
    use std::time::Duration;
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));

    // Create sessions with same status and priority but different elapsed times
    let short = SessionSnapshot {
        session_id: "short".to_string(),
        agent_type: "claudecode".to_string(),
        status: "working".to_string(),
        working_dir: None,
        elapsed_seconds: 10,
        idle_seconds: 5,
        history: vec![],
        closed: false,
        priority: 5,
    };

    let long = SessionSnapshot {
        session_id: "long".to_string(),
        agent_type: "claudecode".to_string(),
        status: "working".to_string(),
        working_dir: None,
        elapsed_seconds: 100,
        idle_seconds: 5,
        history: vec![],
        closed: false,
        priority: 5,
    };

    // Apply in wrong order
    app.apply_update(&short);
    // Wait a tiny bit so the timestamps differ
    std::thread::sleep(Duration::from_millis(10));
    app.apply_update(&long);

    // After sorting, longer elapsed time should be first (same status, same priority)
    assert_eq!(app.sessions[0].session_id, "long");
    assert_eq!(app.sessions[1].session_id, "short");
}

#[test]
fn test_session_sort_combined() {
    use crate::SessionSnapshot;
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));

    // Test combined sorting: status → priority → elapsed
    let sessions = vec![
        SessionSnapshot {
            session_id: "closed-high".to_string(),
            agent_type: "claudecode".to_string(),
            status: "closed".to_string(),
            working_dir: None,
            elapsed_seconds: 100,
            idle_seconds: 5,
            history: vec![],
            closed: true,
            priority: 100,
        },
        SessionSnapshot {
            session_id: "attention-low".to_string(),
            agent_type: "claudecode".to_string(),
            status: "attention".to_string(),
            working_dir: None,
            elapsed_seconds: 50,
            idle_seconds: 5,
            history: vec![],
            closed: false,
            priority: 1,
        },
        SessionSnapshot {
            session_id: "working-high-short".to_string(),
            agent_type: "claudecode".to_string(),
            status: "working".to_string(),
            working_dir: None,
            elapsed_seconds: 10,
            idle_seconds: 5,
            history: vec![],
            closed: false,
            priority: 10,
        },
        SessionSnapshot {
            session_id: "working-high-long".to_string(),
            agent_type: "claudecode".to_string(),
            status: "working".to_string(),
            working_dir: None,
            elapsed_seconds: 100,
            idle_seconds: 5,
            history: vec![],
            closed: false,
            priority: 10,
        },
    ];

    // Apply in random order
    for session in sessions {
        app.apply_update(&session);
    }

    // Expected order:
    // 1. attention-low (status group 0, priority 1)
    // 2. working-high-long (status group 1, priority 10, elapsed 100)
    // 3. working-high-short (status group 1, priority 10, elapsed 10)
    // 4. closed-high (status group 3)
    assert_eq!(app.sessions[0].session_id, "attention-low");
    assert_eq!(app.sessions[1].session_id, "working-high-long");
    assert_eq!(app.sessions[2].session_id, "working-high-short");
    assert_eq!(app.sessions[3].session_id, "closed-high");
}
