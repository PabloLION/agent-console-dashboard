use super::*;
use crate::daemon::store::SessionStore;
use crate::daemon::usage::{UsageFetcher, UsageState};
use crate::IpcCommandKind;
use tokio::sync::broadcast;

fn create_test_state() -> DaemonState {
    let (shutdown_tx, _rx) = broadcast::channel(1);
    DaemonState {
        store: SessionStore::new(),
        start_time: Instant::now(),
        active_connections: Arc::new(AtomicUsize::new(0)),
        socket_path: "/tmp/test.sock".to_string(),
        usage_fetcher: None,
        shutdown_tx: Some(shutdown_tx),
    }
}

#[tokio::test]
async fn test_stop_no_active_sessions_returns_ok() {
    let state = create_test_state();
    let cmd = IpcCommand {
        version: 1,
        cmd: IpcCommandKind::Stop.to_string(),
        session_id: None,
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };

    let response = handle_stop_command(&cmd, &state).await;
    let parsed: IpcResponse = serde_json::from_str(&response).expect("failed to parse response");

    assert!(parsed.ok);
    assert_eq!(
        parsed.data.as_ref().unwrap()["stop_status"].as_str(),
        Some("ok")
    );
}

#[tokio::test]
async fn test_stop_with_active_sessions_requires_confirmation() {
    let state = create_test_state();

    // Add an active session
    state
        .store
        .get_or_create_session(
            "test-session".to_string(),
            AgentType::ClaudeCode,
            Some(std::path::PathBuf::from("/tmp")),
            None,
            Status::Working,
            0,
        )
        .await;

    let cmd = IpcCommand {
        version: 1,
        cmd: IpcCommandKind::Stop.to_string(),
        session_id: None,
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };

    let response = handle_stop_command(&cmd, &state).await;
    let parsed: IpcResponse = serde_json::from_str(&response).expect("failed to parse response");

    assert!(parsed.ok);
    let data = parsed.data.as_ref().unwrap();
    assert_eq!(data["stop_status"].as_str(), Some("confirm_required"));
    assert_eq!(data["active_count"].as_u64(), Some(1));
}

#[tokio::test]
async fn test_stop_with_confirmation_returns_ok() {
    let state = create_test_state();

    // Add an active session
    state
        .store
        .get_or_create_session(
            "test-session".to_string(),
            AgentType::ClaudeCode,
            Some(std::path::PathBuf::from("/tmp")),
            None,
            Status::Working,
            0,
        )
        .await;

    let cmd = IpcCommand {
        version: 1,
        cmd: IpcCommandKind::Stop.to_string(),
        session_id: None,
        status: None,
        working_dir: None,
        confirmed: Some(true),
        priority: None,
    };

    let response = handle_stop_command(&cmd, &state).await;
    let parsed: IpcResponse = serde_json::from_str(&response).expect("failed to parse response");

    assert!(parsed.ok);
    assert_eq!(
        parsed.data.as_ref().unwrap()["stop_status"].as_str(),
        Some("ok")
    );
}

#[tokio::test]
async fn test_stop_with_closed_sessions_returns_ok() {
    let state = create_test_state();

    // Add a closed session (should not require confirmation)
    state
        .store
        .get_or_create_session(
            "closed-session".to_string(),
            AgentType::ClaudeCode,
            Some(std::path::PathBuf::from("/tmp")),
            None,
            Status::Closed,
            0,
        )
        .await;

    let cmd = IpcCommand {
        version: 1,
        cmd: IpcCommandKind::Stop.to_string(),
        session_id: None,
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };

    let response = handle_stop_command(&cmd, &state).await;
    let parsed: IpcResponse = serde_json::from_str(&response).expect("failed to parse response");

    assert!(parsed.ok);
    assert_eq!(
        parsed.data.as_ref().unwrap()["stop_status"].as_str(),
        Some("ok")
    );
}

#[tokio::test]
async fn test_stop_with_inactive_session_returns_ok_without_confirmation() {
    use std::time::Duration;

    let state = create_test_state();

    // Create a session and backdate its last_activity to make it inactive
    let mut session = crate::Session::new(
        "inactive-session".to_string(),
        AgentType::ClaudeCode,
        Some(std::path::PathBuf::from("/tmp")),
    );

    // Backdate last_activity by more than INACTIVE_SESSION_THRESHOLD (3600s)
    session.last_activity = session
        .last_activity
        .checked_sub(INACTIVE_SESSION_THRESHOLD + Duration::from_secs(1))
        .expect("backdate should succeed");

    // Verify the session is inactive before adding to store
    assert!(
        session.is_inactive(INACTIVE_SESSION_THRESHOLD),
        "session should be inactive after backdating"
    );

    // Add the backdated session to the store
    state.store.set(session.session_id.clone(), session).await;

    let cmd = IpcCommand {
        version: 1,
        cmd: IpcCommandKind::Stop.to_string(),
        session_id: None,
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };

    let response = handle_stop_command(&cmd, &state).await;
    let parsed: IpcResponse = serde_json::from_str(&response).expect("failed to parse response");

    assert!(parsed.ok);
    assert_eq!(
        parsed.data.as_ref().unwrap()["stop_status"].as_str(),
        Some("ok"),
        "inactive sessions should not require confirmation"
    );
}

// =============================================================================
// REOPEN command tests
// =============================================================================

#[tokio::test]
async fn test_reopen_command_success() {
    let state = create_test_state();

    // Create and close a session
    state
        .store
        .get_or_create_session(
            "reopen-test".to_string(),
            AgentType::ClaudeCode,
            Some(std::path::PathBuf::from("/tmp")),
            None,
            Status::Working,
            0,
        )
        .await;
    state.store.close_session("reopen-test").await;

    let cmd = IpcCommand {
        version: 1,
        cmd: IpcCommandKind::Reopen.to_string(),
        session_id: Some("reopen-test".to_string()),
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };

    let response = handle_reopen_command(&cmd, &state.store).await;
    let parsed: IpcResponse = serde_json::from_str(&response).expect("failed to parse response");

    if !parsed.ok {
        eprintln!("Error response: {:?}", parsed.error);
    }
    assert!(parsed.ok);
    assert!(parsed.data.is_some());

    // Verify session is active with status=Attention
    let session = state.store.get("reopen-test").await;
    assert!(session.is_some());
    let session = session.unwrap();
    assert_eq!(session.status, Status::Attention);
    assert!(!session.closed);
}

#[tokio::test]
async fn test_reopen_command_not_found() {
    let state = create_test_state();

    let cmd = IpcCommand {
        version: 1,
        cmd: IpcCommandKind::Reopen.to_string(),
        session_id: Some("nonexistent".to_string()),
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };

    let response = handle_reopen_command(&cmd, &state.store).await;
    let parsed: IpcResponse = serde_json::from_str(&response).expect("failed to parse response");

    assert!(!parsed.ok);
    assert!(parsed.error.is_some());
    assert!(parsed.error.unwrap().contains("SESSION_NOT_FOUND"));
}

#[tokio::test]
async fn test_reopen_command_already_active() {
    let state = create_test_state();

    // Create, close, and reopen a session
    state
        .store
        .get_or_create_session(
            "reopen-active".to_string(),
            AgentType::ClaudeCode,
            Some(std::path::PathBuf::from("/tmp")),
            None,
            Status::Working,
            0,
        )
        .await;
    state.store.close_session("reopen-active").await;
    state.store.reopen_session("reopen-active").await.unwrap();

    // Try to reopen again (should fail)
    let cmd = IpcCommand {
        version: 1,
        cmd: IpcCommandKind::Reopen.to_string(),
        session_id: Some("reopen-active".to_string()),
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };

    let response = handle_reopen_command(&cmd, &state.store).await;
    let parsed: IpcResponse = serde_json::from_str(&response).expect("failed to parse response");

    assert!(!parsed.ok);
    assert!(parsed.error.is_some());
}

#[tokio::test]
async fn test_reopen_command_missing_session_id() {
    let state = create_test_state();

    let cmd = IpcCommand {
        version: 1,
        cmd: IpcCommandKind::Reopen.to_string(),
        session_id: None,
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };

    let response = handle_reopen_command(&cmd, &state.store).await;
    let parsed: IpcResponse = serde_json::from_str(&response).expect("failed to parse response");

    assert!(!parsed.ok);
    assert!(parsed.error.is_some());
    assert!(parsed.error.unwrap().contains("REOPEN requires session_id"));
}

// =============================================================================
// DELETE command tests
// =============================================================================

#[tokio::test]
async fn test_delete_command_success() {
    let state = create_test_state();

    // Create a session
    state
        .store
        .get_or_create_session(
            "delete-test".to_string(),
            AgentType::ClaudeCode,
            Some(std::path::PathBuf::from("/tmp")),
            None,
            Status::Working,
            0,
        )
        .await;

    // Verify session exists before deletion
    assert!(state.store.get("delete-test").await.is_some());

    let cmd = IpcCommand {
        version: 1,
        cmd: IpcCommandKind::Delete.to_string(),
        session_id: Some("delete-test".to_string()),
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };

    let response = handle_delete_command(&cmd, &state.store).await;
    let parsed: IpcResponse = serde_json::from_str(&response).expect("failed to parse response");

    assert!(parsed.ok);
    assert!(parsed.data.is_some());

    // Verify session is completely gone from the store
    let session = state.store.get("delete-test").await;
    assert!(session.is_none(), "session should be completely removed");
}

#[tokio::test]
async fn test_delete_command_not_found() {
    let state = create_test_state();

    let cmd = IpcCommand {
        version: 1,
        cmd: IpcCommandKind::Delete.to_string(),
        session_id: Some("nonexistent".to_string()),
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };

    let response = handle_delete_command(&cmd, &state.store).await;
    let parsed: IpcResponse = serde_json::from_str(&response).expect("failed to parse response");

    assert!(!parsed.ok);
    assert!(parsed.error.is_some());
    assert!(parsed.error.unwrap().contains("session not found"));
}

#[tokio::test]
async fn test_delete_command_missing_session_id() {
    let state = create_test_state();

    let cmd = IpcCommand {
        version: 1,
        cmd: IpcCommandKind::Delete.to_string(),
        session_id: None,
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };

    let response = handle_delete_command(&cmd, &state.store).await;
    let parsed: IpcResponse = serde_json::from_str(&response).expect("failed to parse response");

    assert!(!parsed.ok);
    assert!(parsed.error.is_some());
    assert!(parsed.error.unwrap().contains("DELETE requires session_id"));
}

#[tokio::test]
async fn test_delete_command_returns_snapshot() {
    let state = create_test_state();

    // Create a session with specific attributes
    state
        .store
        .get_or_create_session(
            "snapshot-test".to_string(),
            AgentType::ClaudeCode,
            Some(std::path::PathBuf::from("/tmp/test-dir")),
            None,
            Status::Attention,
            42,
        )
        .await;

    let cmd = IpcCommand {
        version: 1,
        cmd: IpcCommandKind::Delete.to_string(),
        session_id: Some("snapshot-test".to_string()),
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };

    let response = handle_delete_command(&cmd, &state.store).await;
    let parsed: IpcResponse = serde_json::from_str(&response).expect("failed to parse response");

    assert!(parsed.ok);
    assert!(parsed.data.is_some());

    // Verify the returned snapshot has correct attributes
    let snapshot: SessionSnapshot =
        serde_json::from_value(parsed.data.unwrap()).expect("failed to parse snapshot");

    assert_eq!(snapshot.session_id, "snapshot-test");
    assert_eq!(snapshot.status, "attention");
    assert_eq!(snapshot.priority, 42);
    assert_eq!(snapshot.working_dir, Some("/tmp/test-dir".to_string()));
}

#[tokio::test]
async fn test_delete_command_other_sessions_unaffected() {
    let state = create_test_state();

    // Create three sessions
    state
        .store
        .get_or_create_session(
            "session-1".to_string(),
            AgentType::ClaudeCode,
            Some(std::path::PathBuf::from("/tmp")),
            None,
            Status::Working,
            0,
        )
        .await;

    state
        .store
        .get_or_create_session(
            "session-2".to_string(),
            AgentType::ClaudeCode,
            Some(std::path::PathBuf::from("/tmp")),
            None,
            Status::Attention,
            0,
        )
        .await;

    state
        .store
        .get_or_create_session(
            "session-3".to_string(),
            AgentType::ClaudeCode,
            Some(std::path::PathBuf::from("/tmp")),
            None,
            Status::Question,
            0,
        )
        .await;

    // Delete session-2
    let cmd = IpcCommand {
        version: 1,
        cmd: IpcCommandKind::Delete.to_string(),
        session_id: Some("session-2".to_string()),
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };

    let response = handle_delete_command(&cmd, &state.store).await;
    let parsed: IpcResponse = serde_json::from_str(&response).expect("failed to parse response");

    assert!(parsed.ok);

    // Verify session-2 is gone
    assert!(state.store.get("session-2").await.is_none());

    // Verify session-1 and session-3 are still present
    let session1 = state.store.get("session-1").await;
    assert!(session1.is_some());
    assert_eq!(session1.unwrap().status, Status::Working);

    let session3 = state.store.get("session-3").await;
    assert!(session3.is_some());
    assert_eq!(session3.unwrap().status, Status::Question);
}

#[tokio::test]
async fn test_delete_closed_session() {
    let state = create_test_state();

    // Create and close a session
    state
        .store
        .get_or_create_session(
            "closed-delete-test".to_string(),
            AgentType::ClaudeCode,
            Some(std::path::PathBuf::from("/tmp")),
            None,
            Status::Working,
            0,
        )
        .await;
    state.store.close_session("closed-delete-test").await;

    // Verify session is closed
    let session = state.store.get("closed-delete-test").await;
    assert!(session.is_some());
    assert!(session.unwrap().closed);

    // Delete the closed session
    let cmd = IpcCommand {
        version: 1,
        cmd: IpcCommandKind::Delete.to_string(),
        session_id: Some("closed-delete-test".to_string()),
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };

    let response = handle_delete_command(&cmd, &state.store).await;
    let parsed: IpcResponse = serde_json::from_str(&response).expect("failed to parse response");

    assert!(parsed.ok);

    // Verify session is completely removed
    assert!(state.store.get("closed-delete-test").await.is_none());
}

// =============================================================================
// SET command usage-refresh tests
// =============================================================================

/// Build a minimal SET IpcCommand for a given session ID and status string.
fn make_set_cmd(session_id: &str, status: &str) -> IpcCommand {
    IpcCommand {
        version: 1,
        cmd: IpcCommandKind::Set.to_string(),
        session_id: Some(session_id.to_string()),
        status: Some(status.to_string()),
        working_dir: None,
        confirmed: None,
        priority: None,
    }
}

#[tokio::test]
async fn test_set_command_without_usage_fetcher_succeeds() {
    // handle_set_command with usage_fetcher=None must still work correctly.
    let store = SessionStore::new();
    let cmd = make_set_cmd("set-no-fetcher", "working");

    let response = handle_set_command(&cmd, &store, None).await;
    let parsed: IpcResponse = serde_json::from_str(&response).expect("failed to parse response");

    assert!(parsed.ok, "SET should succeed without a usage fetcher");
    let snapshot: SessionSnapshot =
        serde_json::from_value(parsed.data.unwrap()).expect("failed to parse snapshot");
    assert_eq!(snapshot.session_id, "set-no-fetcher");
    assert_eq!(snapshot.status, "working");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_set_command_triggers_refresh_when_unavailable() {
    // When usage state is Unavailable, a SET command with a fetcher present
    // should trigger a background refresh (a subscriber receives the broadcast).
    let store = SessionStore::new();
    let fetcher = Arc::new(UsageFetcher::new());
    let mut sub = fetcher.subscribe();

    // Confirm initial state is Unavailable.
    assert!(matches!(
        *fetcher.state().read().await,
        UsageState::Unavailable
    ));

    let cmd = make_set_cmd("set-triggers-refresh", "attention");
    let response = handle_set_command(&cmd, &store, Some(&fetcher)).await;
    let parsed: IpcResponse = serde_json::from_str(&response).expect("failed to parse response");
    assert!(parsed.ok, "SET should succeed");

    // The spawned task must broadcast to the subscriber within a reasonable timeout.
    let result = tokio::time::timeout(std::time::Duration::from_secs(15), sub.recv()).await;
    assert!(
        result.is_ok(),
        "subscriber should receive usage update after SET triggered refresh"
    );
}

#[tokio::test]
async fn test_set_command_no_refresh_when_available() {
    // When usage state is Available, a SET command must NOT trigger a refresh.
    let store = SessionStore::new();
    let fetcher = Arc::new(UsageFetcher::new());

    // Pre-populate state as Available.
    let fake_data = claude_usage::UsageData {
        five_hour: claude_usage::UsagePeriod {
            utilization: 0.0,
            resets_at: None,
        },
        seven_day: claude_usage::UsagePeriod {
            utilization: 0.0,
            resets_at: None,
        },
        seven_day_sonnet: None,
        extra_usage: None,
    };
    *fetcher.state().write().await = UsageState::Available(fake_data);

    let mut sub = fetcher.subscribe();

    let cmd = make_set_cmd("set-no-refresh", "working");
    let response = handle_set_command(&cmd, &store, Some(&fetcher)).await;
    let parsed: IpcResponse = serde_json::from_str(&response).expect("failed to parse response");
    assert!(parsed.ok, "SET should succeed");

    // No broadcast should arrive — timeout is expected.
    let result = tokio::time::timeout(std::time::Duration::from_millis(100), sub.recv()).await;
    assert!(
        result.is_err(),
        "no usage refresh should occur when state is already Available"
    );
}
