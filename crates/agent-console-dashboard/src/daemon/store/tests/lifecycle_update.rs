//! Tests for SessionStore::update_session method.

use super::SessionStore;
use crate::{AgentType, Status};
use std::path::PathBuf;

#[tokio::test]
async fn test_update_session() {
    let store = SessionStore::new();

    let _ = store
        .create_session(
            "update-test".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/home/user/project"),
            None,
        )
        .await;

    let updated = store.update_session("update-test", Status::Attention).await;

    assert!(updated.is_some());
    let session = updated.expect("already checked is_some");
    assert_eq!(session.session_id, "update-test");
    assert_eq!(session.status, Status::Attention);
    assert_eq!(session.history.len(), 1);
    assert_eq!(session.history[0].from, Status::Working);
    assert_eq!(session.history[0].to, Status::Attention);
}

#[tokio::test]
async fn test_update_session_not_found() {
    let store = SessionStore::new();

    let result = store.update_session("nonexistent", Status::Attention).await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_update_session_same_status_no_transition() {
    let store = SessionStore::new();

    let _ = store
        .create_session(
            "same-status".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/tmp/test"),
            None,
        )
        .await;

    let updated = store.update_session("same-status", Status::Working).await;

    assert!(updated.is_some());
    let session = updated.expect("already checked is_some");
    assert_eq!(session.status, Status::Working);
    assert!(session.history.is_empty());
}

#[tokio::test]
async fn test_update_session_multiple_transitions() {
    let store = SessionStore::new();

    let _ = store
        .create_session(
            "multi-transition".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/tmp/test"),
            None,
        )
        .await;

    let _ = store
        .update_session("multi-transition", Status::Attention)
        .await;
    let _ = store
        .update_session("multi-transition", Status::Question)
        .await;
    let result = store
        .update_session("multi-transition", Status::Working)
        .await;

    assert!(result.is_some());
    let session = result.expect("already checked is_some");
    assert_eq!(session.status, Status::Working);
    assert_eq!(session.history.len(), 3);

    assert_eq!(session.history[0].from, Status::Working);
    assert_eq!(session.history[0].to, Status::Attention);
    assert_eq!(session.history[1].from, Status::Attention);
    assert_eq!(session.history[1].to, Status::Question);
    assert_eq!(session.history[2].from, Status::Question);
    assert_eq!(session.history[2].to, Status::Working);
}

#[tokio::test]
async fn test_update_session_persists_in_store() {
    let store = SessionStore::new();

    let _ = store
        .create_session(
            "persist-test".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/tmp/test"),
            None,
        )
        .await;

    let _ = store.update_session("persist-test", Status::Question).await;

    let retrieved = store.get("persist-test").await;
    assert!(retrieved.is_some());
    let session = retrieved.expect("already checked is_some");
    assert_eq!(session.status, Status::Question);
    assert_eq!(session.history.len(), 1);
}

#[tokio::test]
async fn test_update_session_preserves_metadata() {
    let store = SessionStore::new();

    let _ = store
        .create_session(
            "preserve-meta".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/specific/path"),
            Some("claude-session-xyz".to_string()),
        )
        .await;

    let updated = store
        .update_session("preserve-meta", Status::Attention)
        .await;

    assert!(updated.is_some());
    let session = updated.expect("already checked is_some");

    assert_eq!(session.session_id, "preserve-meta");
    assert_eq!(session.agent_type, AgentType::ClaudeCode);
    assert_eq!(session.working_dir, PathBuf::from("/specific/path"));
    assert_eq!(session.status, Status::Attention);
}
