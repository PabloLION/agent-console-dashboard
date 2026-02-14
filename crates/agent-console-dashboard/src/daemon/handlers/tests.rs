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
