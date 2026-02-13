//! Tests for Session struct and related functionality.

use crate::*;

#[test]
fn test_session_new() {
    let session = Session::new(
        "test-session-1".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/home/user/project")),
    );
    assert_eq!(session.session_id, "test-session-1");
    assert_eq!(session.agent_type, AgentType::ClaudeCode);
    assert_eq!(session.status, Status::Working);
    assert_eq!(
        session.working_dir,
        Some(PathBuf::from("/home/user/project"))
    );
    assert!(session.history.is_empty());
    assert!(session.api_usage.is_none());
    assert!(!session.closed);
}

#[test]
fn test_session_default() {
    let session = Session::default();
    assert_eq!(session.session_id, "");
    assert_eq!(session.agent_type, AgentType::ClaudeCode);
    assert_eq!(session.status, Status::Working);
    assert_eq!(session.working_dir, None);
    assert!(session.history.is_empty());
    assert!(session.api_usage.is_none());
    assert!(!session.closed);
}

#[test]
fn test_session_clone() {
    let session = Session::new(
        "clone-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp/test")),
    );
    let cloned = session.clone();
    assert_eq!(cloned.session_id, session.session_id);
    assert_eq!(cloned.agent_type, session.agent_type);
    assert_eq!(cloned.status, session.status);
    assert_eq!(cloned.working_dir, session.working_dir);
}

#[test]
fn test_session_with_all_fields() {
    let mut session = Session::new(
        "full-session".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/home/user/project")),
    );
    session.status = Status::Question;
    session.api_usage = Some(ApiUsage {
        input_tokens: 1000,
        output_tokens: 500,
    });
    session.closed = true;
    session.history.push(StateTransition {
        timestamp: Instant::now(),
        from: Status::Working,
        to: Status::Question,
        duration: Duration::from_secs(60),
    });

    assert_eq!(session.session_id, "full-session");
    assert_eq!(session.status, Status::Question);
    assert!(session.closed);
    assert_eq!(session.api_usage.unwrap().input_tokens, 1000);
    assert_eq!(session.history.len(), 1);
}

#[test]
fn test_session_field_mutability() {
    let mut session = Session::default();
    session.session_id = "updated-id".to_string();
    session.status = Status::Attention;
    session.working_dir = Some(PathBuf::from("/new/path"));
    session.closed = true;

    assert_eq!(session.session_id, "updated-id");
    assert_eq!(session.status, Status::Attention);
    assert_eq!(session.working_dir, Some(PathBuf::from("/new/path")));
    assert!(session.closed);
}

#[test]
fn test_session_set_status_changes_status() {
    let mut session = Session::new(
        "status-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp")),
    );
    assert_eq!(session.status, Status::Working);

    session.set_status(Status::Attention);
    assert_eq!(session.status, Status::Attention);
}

#[test]
fn test_session_set_status_records_transition() {
    let mut session = Session::new(
        "transition-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp")),
    );
    assert!(session.history.is_empty());

    session.set_status(Status::Question);

    assert_eq!(session.history.len(), 1);
    assert_eq!(session.history[0].from, Status::Working);
    assert_eq!(session.history[0].to, Status::Question);
}

#[test]
fn test_session_set_status_same_status_no_transition() {
    let mut session = Session::new(
        "same-status-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp")),
    );

    // Setting to the same status should not record a transition
    session.set_status(Status::Working);
    assert!(session.history.is_empty());
    assert_eq!(session.status, Status::Working);
}

#[test]
fn test_session_set_status_same_status_resets_since() {
    let mut session = Session::new(
        "same-status-reset-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp")),
    );
    let original_since = session.since;

    // Small delay to ensure time difference
    std::thread::sleep(Duration::from_millis(10));

    // Setting same status should reset 'since'
    session.set_status(Status::Working);

    // History should still be empty (no transition recorded)
    assert!(session.history.is_empty());
    // But 'since' should be reset to a later time
    assert!(
        session.since > original_since,
        "since should be reset on same-status transition"
    );
}

#[test]
fn test_session_set_status_multiple_transitions() {
    let mut session = Session::new(
        "multi-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp")),
    );

    session.set_status(Status::Attention);
    session.set_status(Status::Question);
    session.set_status(Status::Closed);

    assert_eq!(session.history.len(), 3);
    assert_eq!(session.history[0].from, Status::Working);
    assert_eq!(session.history[0].to, Status::Attention);
    assert_eq!(session.history[1].from, Status::Attention);
    assert_eq!(session.history[1].to, Status::Question);
    assert_eq!(session.history[2].from, Status::Question);
    assert_eq!(session.history[2].to, Status::Closed);
}

#[test]
fn test_session_set_status_closed_to_working_clears_closed_flag() {
    let mut session = Session::new(
        "latch-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp")),
    );

    // Close the session
    session.set_status(Status::Closed);
    assert!(session.closed, "closed flag should be set");

    // Re-activate the session
    session.set_status(Status::Working);
    assert!(
        !session.closed,
        "closed flag should be cleared on re-activation"
    );
    assert_eq!(session.status, Status::Working);
}

#[test]
fn test_session_set_status_updates_since() {
    let mut session = Session::new(
        "since-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp")),
    );
    let original_since = session.since;

    // Small delay to ensure time difference
    std::thread::sleep(Duration::from_millis(10));

    session.set_status(Status::Attention);

    // 'since' should be updated to a later time
    assert!(session.since > original_since);
}

#[test]
fn test_session_set_status_updates_last_activity_on_change() {
    let mut session = Session::new(
        "activity-change-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp")),
    );
    let original = session.last_activity;

    std::thread::sleep(Duration::from_millis(10));
    session.set_status(Status::Attention);

    assert!(session.last_activity > original);
}

#[test]
fn test_session_set_status_updates_last_activity_on_same_status() {
    let mut session = Session::new(
        "activity-same-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp")),
    );
    let original_activity = session.last_activity;
    let original_since = session.since;

    std::thread::sleep(Duration::from_millis(10));

    // Same status: both last_activity and since should advance (timer reset)
    session.set_status(Status::Working);

    assert!(
        session.last_activity > original_activity,
        "last_activity should advance on same-status call"
    );
    assert!(
        session.since > original_since,
        "since should advance on same-status call (timer reset)"
    );
}

#[test]
fn test_session_is_inactive_when_old() {
    let mut session = Session::new(
        "inactive-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp")),
    );
    let threshold = Duration::from_secs(3600);

    // Fresh session is not inactive
    assert!(!session.is_inactive(threshold));

    // Backdate last_activity to 2 hours ago
    session.last_activity = session
        .last_activity
        .checked_sub(Duration::from_secs(7200))
        .expect("backdate should succeed");
    assert!(session.is_inactive(threshold));
}

#[test]
fn test_session_is_inactive_excludes_closed() {
    let mut session = Session::new(
        "closed-inactive-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp")),
    );
    let threshold = Duration::from_secs(3600);

    // Backdate and close
    session.last_activity = session
        .last_activity
        .checked_sub(Duration::from_secs(7200))
        .expect("backdate should succeed");
    session.set_status(Status::Closed);

    assert!(
        !session.is_inactive(threshold),
        "closed sessions are never inactive"
    );
}

#[test]
fn test_session_set_status_transition_has_duration() {
    let mut session = Session::new(
        "duration-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp")),
    );

    // Small delay to ensure measurable duration
    std::thread::sleep(Duration::from_millis(10));

    session.set_status(Status::Question);

    assert_eq!(session.history.len(), 1);
    // Duration should be at least 10ms
    assert!(session.history[0].duration >= Duration::from_millis(10));
}

#[test]
fn test_session_history_multiple_entries() {
    let mut session = Session::default();

    // Add multiple history entries
    for i in 0..5 {
        session.history.push(StateTransition {
            timestamp: Instant::now(),
            from: Status::Working,
            to: Status::Question,
            duration: Duration::from_secs(i as u64),
        });
    }

    assert_eq!(session.history.len(), 5);
    assert_eq!(session.history[0].duration, Duration::from_secs(0));
    assert_eq!(session.history[4].duration, Duration::from_secs(4));
}

#[test]
fn test_session_debug_format() {
    let session = Session::new(
        "debug-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp")),
    );
    let debug_str = format!("{:?}", session);
    // Debug output should contain the session ID
    assert!(debug_str.contains("debug-test"));
    assert!(debug_str.contains("ClaudeCode"));
}
