//! Tests for SessionStore::reopen_session method.

use super::SessionStore;
use crate::{AgentType, Status};
use std::path::PathBuf;

// =============================================================================
// reopen_session tests
// =============================================================================

#[tokio::test]
async fn test_reopen_session_success() {
    let store = SessionStore::new();

    // Create and close a session
    let _ = store
        .get_or_create_session(
            "reopen-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
            None,
            Status::Working,
            0,
        )
        .await;
    store.close_session("reopen-test").await;

    // Reopen the session
    let result = store.reopen_session("reopen-test").await;
    assert!(result.is_ok());

    let session = result.unwrap();
    assert_eq!(session.session_id, "reopen-test");
    assert_eq!(session.status, Status::Attention);
    assert!(!session.closed);
}

#[tokio::test]
async fn test_reopen_session_not_found() {
    let store = SessionStore::new();

    let result = store.reopen_session("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_reopen_session_already_active() {
    let store = SessionStore::new();

    // Create and close a session
    let _ = store
        .get_or_create_session(
            "reopen-active".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
            None,
            Status::Working,
            0,
        )
        .await;
    store.close_session("reopen-active").await;

    // Reopen once (should succeed)
    let result1 = store.reopen_session("reopen-active").await;
    assert!(result1.is_ok());

    // Try to reopen again (should fail - already active)
    let result2 = store.reopen_session("reopen-active").await;
    assert!(result2.is_err());
}

#[tokio::test]
async fn test_reopen_session_removes_from_closed_queue() {
    let store = SessionStore::new();

    // Create and close a session
    let _ = store
        .get_or_create_session(
            "reopen-queue".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
            None,
            Status::Working,
            0,
        )
        .await;
    store.close_session("reopen-queue").await;

    // Verify it's in closed queue
    let closed = store.get_closed("reopen-queue").await;
    assert!(closed.is_some());

    // Reopen the session
    let result = store.reopen_session("reopen-queue").await;
    assert!(result.is_ok());

    // Verify it's no longer in closed queue
    let closed_after = store.get_closed("reopen-queue").await;
    assert!(closed_after.is_none());
}

#[tokio::test]
async fn test_reopen_session_adds_to_active_sessions() {
    let store = SessionStore::new();

    // Create and close a session
    let _ = store
        .get_or_create_session(
            "reopen-active-add".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
            None,
            Status::Working,
            0,
        )
        .await;
    store.close_session("reopen-active-add").await;

    // Reopen the session
    let result = store.reopen_session("reopen-active-add").await;
    assert!(result.is_ok());

    // Verify it's in active sessions
    let retrieved = store.get("reopen-active-add").await;
    assert!(retrieved.is_some());
    let session = retrieved.unwrap();
    assert!(!session.closed);
    assert_eq!(session.status, Status::Attention);
}

#[tokio::test]
async fn test_reopen_session_preserves_metadata() {
    let store = SessionStore::new();

    // Create and close a session with specific metadata
    let _ = store
        .get_or_create_session(
            "reopen-metadata".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/specific/path")),
            None,
            Status::Working,
            0,
        )
        .await;
    store.close_session("reopen-metadata").await;

    // Reopen the session
    let result = store.reopen_session("reopen-metadata").await;
    assert!(result.is_ok());

    let session = result.unwrap();
    assert_eq!(session.session_id, "reopen-metadata");
    assert_eq!(session.agent_type, AgentType::ClaudeCode);
    assert_eq!(session.working_dir, Some(PathBuf::from("/specific/path")));
    assert_eq!(session.status, Status::Attention);
    assert!(!session.closed);
}

#[tokio::test]
async fn test_reopen_session_resets_priority() {
    let store = SessionStore::new();

    // Create session with priority 10
    let _ = store
        .get_or_create_session(
            "reopen-priority".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
            None,
            Status::Working,
            10,
        )
        .await;
    store.close_session("reopen-priority").await;

    // Reopen the session
    let result = store.reopen_session("reopen-priority").await;
    assert!(result.is_ok());

    let session = result.unwrap();
    // Priority should be reset to 0 on reopen
    assert_eq!(session.priority, 0);
}

#[tokio::test]
async fn test_reopen_session_appears_in_list_all() {
    let store = SessionStore::new();

    // Create two sessions, close one
    let _ = store
        .get_or_create_session(
            "reopen-list-1".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
            None,
            Status::Working,
            0,
        )
        .await;
    let _ = store
        .get_or_create_session(
            "reopen-list-2".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
            None,
            Status::Working,
            0,
        )
        .await;
    store.close_session("reopen-list-1").await;

    // List should show both (one closed, one active)
    let sessions = store.list_all().await;
    assert_eq!(sessions.len(), 2);

    // Reopen the closed session
    let result = store.reopen_session("reopen-list-1").await;
    assert!(result.is_ok());

    // List should still show both, but now both are active
    let sessions = store.list_all().await;
    assert_eq!(sessions.len(), 2);

    let session1 = sessions.iter().find(|s| s.session_id == "reopen-list-1");
    assert!(session1.is_some());
    assert!(!session1.unwrap().closed);

    let session2 = sessions.iter().find(|s| s.session_id == "reopen-list-2");
    assert!(session2.is_some());
    assert!(!session2.unwrap().closed);
}
