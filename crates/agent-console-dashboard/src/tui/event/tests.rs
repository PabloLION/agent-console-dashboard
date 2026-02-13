use super::*;
use crate::{AgentType, Session};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use std::path::PathBuf;

fn make_key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn make_app_with_sessions(count: usize) -> App {
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));
    for i in 0..count {
        app.sessions.push(Session::new(
            format!("session-{}", i),
            AgentType::ClaudeCode,
            Some(PathBuf::from(format!("/home/user/project-{}", i))),
        ));
    }
    app.init_selection();
    app
}

#[test]
fn test_should_quit_on_q() {
    assert!(should_quit(make_key(
        KeyCode::Char('q'),
        KeyModifiers::NONE
    )));
}

#[test]
fn test_should_quit_on_ctrl_c() {
    assert!(should_quit(make_key(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL
    )));
}

#[test]
fn test_should_not_quit_on_other_keys() {
    assert!(!should_quit(make_key(
        KeyCode::Char('a'),
        KeyModifiers::NONE
    )));
    assert!(!should_quit(make_key(KeyCode::Enter, KeyModifiers::NONE)));
    assert!(!should_quit(make_key(KeyCode::Esc, KeyModifiers::NONE)));
}

#[test]
fn test_event_handler_creation() {
    let handler = EventHandler::new(Duration::from_millis(250));
    assert_eq!(handler.tick_rate, Duration::from_millis(250));
}

#[test]
fn test_event_debug_format() {
    let event = Event::Tick;
    let debug = format!("{:?}", event);
    assert!(debug.contains("Tick"));
}

#[test]
fn test_event_resize_variant() {
    let event = Event::Resize(80, 24);
    match event {
        Event::Resize(w, h) => {
            assert_eq!(w, 80);
            assert_eq!(h, 24);
        }
        _ => panic!("expected Resize variant"),
    }
}

// --- handle_key_event tests ---

#[test]
fn test_handle_key_j_selects_next() {
    let mut app = make_app_with_sessions(3);
    assert_eq!(app.selected_index, Some(0));
    let action = handle_key_event(&mut app, make_key(KeyCode::Char('j'), KeyModifiers::NONE));
    assert_eq!(action, Action::None);
    assert_eq!(app.selected_index, Some(1));
}

#[test]
fn test_handle_key_k_selects_previous() {
    let mut app = make_app_with_sessions(3);
    app.selected_index = Some(2);
    let action = handle_key_event(&mut app, make_key(KeyCode::Char('k'), KeyModifiers::NONE));
    assert_eq!(action, Action::None);
    assert_eq!(app.selected_index, Some(1));
}

#[test]
fn test_handle_key_down_selects_next() {
    let mut app = make_app_with_sessions(3);
    let action = handle_key_event(&mut app, make_key(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(action, Action::None);
    assert_eq!(app.selected_index, Some(1));
}

#[test]
fn test_handle_key_up_selects_previous() {
    let mut app = make_app_with_sessions(3);
    app.selected_index = Some(2);
    let action = handle_key_event(&mut app, make_key(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(action, Action::None);
    assert_eq!(app.selected_index, Some(1));
}

#[test]
fn test_handle_key_q_quits() {
    let mut app = make_app_with_sessions(1);
    let action = handle_key_event(&mut app, make_key(KeyCode::Char('q'), KeyModifiers::NONE));
    assert_eq!(action, Action::Quit);
}

#[test]
fn test_handle_key_ctrl_c_quits() {
    let mut app = make_app_with_sessions(1);
    let action = handle_key_event(
        &mut app,
        make_key(KeyCode::Char('c'), KeyModifiers::CONTROL),
    );
    assert_eq!(action, Action::Quit);
}

#[test]
fn test_handle_key_unknown_returns_none() {
    let mut app = make_app_with_sessions(1);
    let noop_keys = [KeyCode::Char('a'), KeyCode::Char('z'), KeyCode::Tab];
    for code in noop_keys {
        let action = handle_key_event(&mut app, make_key(code, KeyModifiers::NONE));
        assert_eq!(action, Action::None, "expected None for {:?}", code);
    }
}

#[test]
fn test_handle_enter_opens_detail() {
    let mut app = make_app_with_sessions(3);
    let action = handle_key_event(&mut app, make_key(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(action, Action::OpenDetail(0));
}

#[test]
fn test_handle_enter_no_selection_returns_none() {
    let mut app = App::new(PathBuf::from("/tmp/test.sock"));
    let action = handle_key_event(&mut app, make_key(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(action, Action::None);
}

#[test]
fn test_handle_r_resurrects_closed_session() {
    let mut app = make_app_with_sessions(1);
    app.sessions[0].status = crate::Status::Closed;
    let action = handle_key_event(&mut app, make_key(KeyCode::Char('r'), KeyModifiers::NONE));
    assert_eq!(action, Action::Resurrect("session-0".to_string()));
}

#[test]
fn test_handle_r_on_working_session_returns_none() {
    let mut app = make_app_with_sessions(1);
    let action = handle_key_event(&mut app, make_key(KeyCode::Char('r'), KeyModifiers::NONE));
    assert_eq!(action, Action::None);
}

#[test]
fn test_handle_d_removes_session() {
    let mut app = make_app_with_sessions(1);
    let action = handle_key_event(&mut app, make_key(KeyCode::Char('d'), KeyModifiers::NONE));
    assert_eq!(action, Action::Remove("session-0".to_string()));
}

#[test]
fn test_handle_layout_keys() {
    let mut app = make_app_with_sessions(1);
    assert_eq!(
        handle_key_event(&mut app, make_key(KeyCode::Char('1'), KeyModifiers::NONE)),
        Action::SwitchLayout(1)
    );
    assert_eq!(
        handle_key_event(&mut app, make_key(KeyCode::Char('4'), KeyModifiers::NONE)),
        Action::SwitchLayout(4)
    );
}

#[test]
fn test_handle_esc_returns_back() {
    let mut app = make_app_with_sessions(1);
    let action = handle_key_event(&mut app, make_key(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(action, Action::Back);
}

#[test]
fn test_handle_key_navigation_integration() {
    let mut app = make_app_with_sessions(5);
    // Navigate down 3 times
    for _ in 0..3 {
        handle_key_event(&mut app, make_key(KeyCode::Char('j'), KeyModifiers::NONE));
    }
    assert_eq!(app.selected_index, Some(3));
    // Navigate up 2 times
    for _ in 0..2 {
        handle_key_event(&mut app, make_key(KeyCode::Char('k'), KeyModifiers::NONE));
    }
    assert_eq!(app.selected_index, Some(1));
    // Verify selected session
    assert_eq!(
        app.selected_session()
            .expect("should have selected session")
            .session_id,
        "session-1"
    );
}

#[test]
fn test_action_debug_and_equality() {
    assert_eq!(Action::None, Action::None);
    assert_eq!(Action::Quit, Action::Quit);
    assert_ne!(Action::None, Action::Quit);
    let debug = format!("{:?}", Action::Quit);
    assert!(debug.contains("Quit"));
}

// --- Detail view key handling tests ---

#[test]
fn test_detail_view_esc_returns_back() {
    let mut app = make_app_with_sessions(3);
    app.view = crate::tui::app::View::Detail {
        session_index: 0,
        history_scroll: 0,
    };
    let action = handle_key_event(&mut app, make_key(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(action, Action::Back);
}

#[test]
fn test_detail_view_q_quits() {
    let mut app = make_app_with_sessions(1);
    app.view = crate::tui::app::View::Detail {
        session_index: 0,
        history_scroll: 0,
    };
    let action = handle_key_event(&mut app, make_key(KeyCode::Char('q'), KeyModifiers::NONE));
    assert_eq!(action, Action::Quit);
}

#[test]
fn test_detail_view_r_resurrects_closed() {
    let mut app = make_app_with_sessions(1);
    app.sessions[0].status = crate::Status::Closed;
    app.view = crate::tui::app::View::Detail {
        session_index: 0,
        history_scroll: 0,
    };
    let action = handle_key_event(&mut app, make_key(KeyCode::Char('r'), KeyModifiers::NONE));
    assert_eq!(action, Action::Resurrect("session-0".to_string()));
}

#[test]
fn test_detail_view_r_on_working_returns_none() {
    let mut app = make_app_with_sessions(1);
    app.view = crate::tui::app::View::Detail {
        session_index: 0,
        history_scroll: 0,
    };
    let action = handle_key_event(&mut app, make_key(KeyCode::Char('r'), KeyModifiers::NONE));
    assert_eq!(action, Action::None);
}

#[test]
fn test_detail_view_c_removes() {
    let mut app = make_app_with_sessions(1);
    app.view = crate::tui::app::View::Detail {
        session_index: 0,
        history_scroll: 0,
    };
    let action = handle_key_event(&mut app, make_key(KeyCode::Char('c'), KeyModifiers::NONE));
    assert_eq!(action, Action::Remove("session-0".to_string()));
}

#[test]
fn test_detail_view_j_scrolls_down() {
    let mut app = make_app_with_sessions(1);
    app.view = crate::tui::app::View::Detail {
        session_index: 0,
        history_scroll: 0,
    };
    let action = handle_key_event(&mut app, make_key(KeyCode::Char('j'), KeyModifiers::NONE));
    assert_eq!(action, Action::ScrollHistoryDown);
}

#[test]
fn test_detail_view_k_scrolls_up() {
    let mut app = make_app_with_sessions(1);
    app.view = crate::tui::app::View::Detail {
        session_index: 0,
        history_scroll: 0,
    };
    let action = handle_key_event(&mut app, make_key(KeyCode::Char('k'), KeyModifiers::NONE));
    assert_eq!(action, Action::ScrollHistoryUp);
}

#[test]
fn test_detail_view_layout_keys_ignored() {
    let mut app = make_app_with_sessions(1);
    app.view = crate::tui::app::View::Detail {
        session_index: 0,
        history_scroll: 0,
    };
    // '1' should not switch layout in detail view
    let action = handle_key_event(&mut app, make_key(KeyCode::Char('1'), KeyModifiers::NONE));
    assert_eq!(action, Action::None);
}
