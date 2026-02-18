use super::*;
use crate::tui::app::App;
use crate::tui::test_utils::{
    find_row_with_text, make_session as make_test_session_with_dir, render_dashboard_to_buffer,
};
use std::time::Instant;

// --- Detail Panel Tests (acd-211, acd-bbh, acd-4sq) ---

#[test]
fn test_detail_renders_below_session_list_not_centered() {
    let mut app = App::new(PathBuf::from("/tmp/test.sock"), None);
    app.sessions.push(make_test_session_with_dir(
        "test-session",
        Status::Working,
        Some(PathBuf::from("/tmp")),
    ));
    app.init_selection();
    app.open_detail(0);

    let buffer = render_dashboard_to_buffer(&mut app, 80, 30);

    let status_row = find_row_with_text(&buffer, "Status:").expect("should find Status: in detail");

    assert!(
        status_row > 10,
        "Detail panel should be in bottom section, not centered: row={}",
        status_row
    );
}

#[test]
fn test_detail_section_shows_session_status() {
    let mut app = App::new(PathBuf::from("/tmp/test.sock"), None);
    let mut session = make_test_session_with_dir(
        "status-test",
        Status::Attention,
        Some(PathBuf::from("/tmp")),
    );
    session.status = Status::Attention;
    app.sessions.push(session);
    app.init_selection();
    app.open_detail(0);

    let buffer = render_dashboard_to_buffer(&mut app, 80, 30);

    assert!(
        find_row_with_text(&buffer, "Status:").is_some(),
        "Detail should show 'Status:' label"
    );

    assert!(
        find_row_with_text(&buffer, "attention").is_some(),
        "Detail should show status value"
    );
}

#[test]
fn test_detail_section_shows_working_directory() {
    let mut app = App::new(PathBuf::from("/tmp/test.sock"), None);
    app.sessions.push(make_test_session_with_dir(
        "dir-test",
        Status::Working,
        Some(PathBuf::from("/home/user/my-project")),
    ));
    app.init_selection();
    app.open_detail(0);

    let buffer = render_dashboard_to_buffer(&mut app, 80, 30);

    assert!(
        find_row_with_text(&buffer, "Dir:").is_some(),
        "Detail should show 'Dir:' label"
    );

    assert!(
        find_row_with_text(&buffer, "my-project").is_some(),
        "Detail should show directory path"
    );
}

#[test]
fn test_detail_section_shows_session_id() {
    let mut app = App::new(PathBuf::from("/tmp/test.sock"), None);
    app.sessions.push(make_test_session_with_dir(
        "unique-session-id-12345",
        Status::Working,
        Some(PathBuf::from("/tmp")),
    ));
    app.init_selection();
    app.open_detail(0);

    let buffer = render_dashboard_to_buffer(&mut app, 80, 30);

    assert!(
        find_row_with_text(&buffer, "ID:").is_some(),
        "Detail should show 'ID:' label"
    );

    assert!(
        find_row_with_text(&buffer, "unique-session-id").is_some(),
        "Detail should show session ID"
    );
}

#[test]
fn test_detail_shows_action_hints() {
    let mut session = make_session("hints-test");
    session.status = Status::Working;

    let lines = build_detail_lines(&session, 60, 0, Instant::now(), true);
    let text: String = lines
        .iter()
        .flat_map(|line| line.spans.iter().map(|span| span.content.as_ref()))
        .collect();

    assert!(
        text.contains("[ESC] Back"),
        "Detail should show ESC action hint: '{}'",
        text
    );

    assert!(
        text.contains("[C]lose"),
        "Detail should show Close action hint: '{}'",
        text
    );
}

#[test]
fn test_detail_closed_session_shows_resurrect() {
    let mut session = make_session("closed-test");
    session.status = Status::Closed;
    session.closed = true;

    let lines = build_detail_lines(&session, 60, 0, Instant::now(), true);
    let text: String = lines
        .iter()
        .flat_map(|line| line.spans.iter().map(|span| span.content.as_ref()))
        .collect();

    assert!(
        text.contains("[R]esurrect"),
        "Closed session detail should show Resurrect hint: '{}'",
        text
    );
}

#[test]
fn test_detail_unknown_dir_shows_error_not_unknown() {
    let session = Session::new("error-dir-test".to_string(), AgentType::ClaudeCode, None);

    let lines = build_detail_lines(&session, 60, 0, Instant::now(), true);
    let text: String = lines
        .iter()
        .flat_map(|line| line.spans.iter().map(|span| span.content.as_ref()))
        .collect();

    assert!(
        text.contains("<error>"),
        "Unknown dir should show '<error>': '{}'",
        text
    );

    assert!(
        !text.contains("unknown"),
        "Unknown dir should not show 'unknown': '{}'",
        text
    );
}

#[test]
fn test_detail_normal_dir_shows_path() {
    let session = make_session("normal-dir-test");

    let lines = build_detail_lines(&session, 60, 0, Instant::now(), true);
    let text: String = lines
        .iter()
        .flat_map(|line| line.spans.iter().map(|span| span.content.as_ref()))
        .collect();

    assert!(
        text.contains("project-a"),
        "Normal dir should show path: '{}'",
        text
    );

    assert!(
        !text.contains("<error>"),
        "Normal dir should not show '<error>': '{}'",
        text
    );
}

#[test]
fn test_detail_no_history_shows_placeholder() {
    let session = make_session("no-history-test");

    let lines = build_detail_lines(&session, 60, 0, Instant::now(), true);
    let text: String = lines
        .iter()
        .flat_map(|line| line.spans.iter().map(|span| span.content.as_ref()))
        .collect();

    assert!(
        text.contains("(no transitions)"),
        "No history should show placeholder: '{}'",
        text
    );
}

#[test]
fn test_detail_history_shows_transitions() {
    let mut session = make_session("history-test");
    let now = Instant::now();

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

    assert!(
        text.contains("→"),
        "History should show transition arrow: '{}'",
        text
    );

    assert!(
        text.contains("working"),
        "History should show from status: '{}'",
        text
    );

    assert!(
        text.contains("attention"),
        "History should show to status: '{}'",
        text
    );
}

#[test]
fn test_detail_history_scroll_shows_entry_count() {
    let mut session = make_session("scroll-test");
    let now = Instant::now();

    for i in 0..10 {
        session.history.push(StateTransition {
            timestamp: now - Duration::from_secs(60 * (10 - i)),
            from: Status::Working,
            to: Status::Attention,
            duration: Duration::from_secs(30),
        });
    }

    let lines = build_detail_lines(&session, 60, 0, now, true);
    let text: String = lines
        .iter()
        .flat_map(|line| line.spans.iter().map(|span| span.content.as_ref()))
        .collect();

    assert!(
        text.contains("entries"),
        "History scroll should show entry count: '{}'",
        text
    );

    assert!(
        text.contains("[") && text.contains("/") && text.contains("]"),
        "History scroll should show [X/Y entries] format: '{}'",
        text
    );
}
