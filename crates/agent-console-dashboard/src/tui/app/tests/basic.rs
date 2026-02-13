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
    assert_eq!(
        app.view,
        View::Detail {
            session_index: 1,
            history_scroll: 0,
        }
    );
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
    app.open_detail(0);
    app.close_detail();
    assert_eq!(app.view, View::Dashboard);
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
    app.open_detail(0);
    app.scroll_history_down();
    assert_eq!(
        app.view,
        View::Detail {
            session_index: 0,
            history_scroll: 1,
        }
    );
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
    app.view = View::Detail {
        session_index: 0,
        history_scroll: 3,
    };
    app.scroll_history_up();
    assert_eq!(
        app.view,
        View::Detail {
            session_index: 0,
            history_scroll: 2,
        }
    );
}

#[test]
fn test_scroll_history_up_clamps_at_zero() {
    let mut app = make_app_with_sessions(1);
    app.open_detail(0);
    app.scroll_history_up();
    assert_eq!(
        app.view,
        View::Detail {
            session_index: 0,
            history_scroll: 0,
        }
    );
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
