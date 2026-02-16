use super::*;
use crate::daemon::store::SessionStore;
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
        cmd: "STOP".to_string(),
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
        cmd: "STOP".to_string(),
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
        cmd: "STOP".to_string(),
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
        cmd: "STOP".to_string(),
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
        cmd: "STOP".to_string(),
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
        cmd: "REOPEN".to_string(),
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
        cmd: "REOPEN".to_string(),
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
        cmd: "REOPEN".to_string(),
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
        cmd: "REOPEN".to_string(),
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
        cmd: "DELETE".to_string(),
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
        cmd: "DELETE".to_string(),
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
        cmd: "DELETE".to_string(),
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
        cmd: "DELETE".to_string(),
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
        cmd: "DELETE".to_string(),
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
        cmd: "DELETE".to_string(),
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
