//! Tests for SessionStore::create_session method.

use super::{create_test_session, SessionStore};
use crate::{AgentType, StoreError};
use std::path::PathBuf;

#[tokio::test]
async fn test_create_session() {
    let store = SessionStore::new();

    let result = store
        .create_session(
            "new-session".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/home/user/project"),
            Some("claude-session-123".to_string()),
        )
        .await;

    assert!(result.is_ok());
    let session = result.expect("already checked is_ok");
    assert_eq!(session.id, "new-session");
    assert_eq!(session.agent_type, AgentType::ClaudeCode);
    assert_eq!(session.working_dir, PathBuf::from("/home/user/project"));
    assert_eq!(session.session_id, Some("claude-session-123".to_string()));
    assert!(!session.closed);

    let retrieved = store.get("new-session").await;
    assert!(retrieved.is_some());
    assert_eq!(
        retrieved.expect("already checked is_some").id,
        "new-session"
    );
}

#[tokio::test]
async fn test_create_session_without_session_id() {
    let store = SessionStore::new();

    let result = store
        .create_session(
            "no-session-id".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/tmp/test"),
            None,
        )
        .await;

    assert!(result.is_ok());
    let session = result.expect("already checked is_ok");
    assert_eq!(session.id, "no-session-id");
    assert!(session.session_id.is_none());
}

#[tokio::test]
async fn test_create_session_already_exists_error() {
    let store = SessionStore::new();

    let result1 = store
        .create_session(
            "duplicate-id".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/path/1"),
            None,
        )
        .await;
    assert!(result1.is_ok());

    let result2 = store
        .create_session(
            "duplicate-id".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/path/2"),
            None,
        )
        .await;

    assert!(result2.is_err());
    match result2.expect_err("already checked is_err") {
        StoreError::SessionExists(id) => {
            assert_eq!(id, "duplicate-id");
        }
        other => panic!("Expected SessionExists error, got: {:?}", other),
    }

    let retrieved = store.get("duplicate-id").await.expect("session should exist");
    assert_eq!(retrieved.working_dir, PathBuf::from("/path/1"));
}

#[tokio::test]
async fn test_create_session_explicit_vs_set() {
    let store = SessionStore::new();

    let session1 = create_test_session("test-id");
    store.set("test-id".to_string(), session1).await;

    let result = store
        .create_session(
            "test-id".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/new/path"),
            None,
        )
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_session_multiple_unique() {
    let store = SessionStore::new();

    for i in 0..5 {
        let result = store
            .create_session(
                format!("unique-{}", i),
                AgentType::ClaudeCode,
                PathBuf::from(format!("/path/{}", i)),
                None,
            )
            .await;
        assert!(result.is_ok(), "Failed to create session unique-{}", i);
    }

    let sessions = store.list_all().await;
    assert_eq!(sessions.len(), 5);
}
