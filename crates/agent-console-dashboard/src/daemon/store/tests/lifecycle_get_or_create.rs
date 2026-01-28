//! Tests for SessionStore::get_or_create_session method.

use super::{create_test_session, SessionStore};
use crate::AgentType;
use std::path::PathBuf;

#[tokio::test]
async fn test_get_or_create_session_creates_new() {
    let store = SessionStore::new();

    let session = store
        .get_or_create_session(
            "new-session".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/home/user/project"),
            Some("claude-session-123".to_string()),
        )
        .await;

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
async fn test_get_or_create_session_returns_existing() {
    let store = SessionStore::new();

    let original = store
        .get_or_create_session(
            "existing-session".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/original/path"),
            Some("original-session-id".to_string()),
        )
        .await;

    let retrieved = store
        .get_or_create_session(
            "existing-session".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/different/path"),
            Some("different-session-id".to_string()),
        )
        .await;

    assert_eq!(retrieved.id, "existing-session");
    assert_eq!(retrieved.working_dir, PathBuf::from("/original/path"));
    assert_eq!(
        retrieved.session_id,
        Some("original-session-id".to_string())
    );

    let from_store = store
        .get("existing-session")
        .await
        .expect("session should exist");
    assert_eq!(from_store.working_dir, original.working_dir);
    assert_eq!(from_store.session_id, original.session_id);
}

#[tokio::test]
async fn test_get_or_create_session_without_session_id() {
    let store = SessionStore::new();

    let session = store
        .get_or_create_session(
            "no-session-id".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/tmp/test"),
            None,
        )
        .await;

    assert_eq!(session.id, "no-session-id");
    assert!(session.session_id.is_none());
}

#[tokio::test]
async fn test_get_or_create_session_after_set() {
    let store = SessionStore::new();

    let session1 = create_test_session("test-id");
    store.set("test-id".to_string(), session1).await;

    let session2 = store
        .get_or_create_session(
            "test-id".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/new/path"),
            None,
        )
        .await;

    assert_eq!(session2.id, "test-id");
    assert_eq!(session2.working_dir, PathBuf::from("/home/user/test-id"));
}

#[tokio::test]
async fn test_get_or_create_session_after_create_session() {
    let store = SessionStore::new();

    let result = store
        .create_session(
            "test-id".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/original/path"),
            Some("original-id".to_string()),
        )
        .await;
    assert!(result.is_ok());

    let session = store
        .get_or_create_session(
            "test-id".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/new/path"),
            Some("new-id".to_string()),
        )
        .await;

    assert_eq!(session.id, "test-id");
    assert_eq!(session.working_dir, PathBuf::from("/original/path"));
    assert_eq!(session.session_id, Some("original-id".to_string()));
}

#[tokio::test]
async fn test_get_or_create_session_multiple_unique() {
    let store = SessionStore::new();

    for i in 0..5 {
        let session = store
            .get_or_create_session(
                format!("session-{}", i),
                AgentType::ClaudeCode,
                PathBuf::from(format!("/path/{}", i)),
                None,
            )
            .await;
        assert_eq!(session.id, format!("session-{}", i));
    }

    let sessions = store.list_all().await;
    assert_eq!(sessions.len(), 5);
}

#[tokio::test]
async fn test_get_or_create_session_idempotent() {
    let store = SessionStore::new();

    let session1 = store
        .get_or_create_session(
            "idempotent-test".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/path/1"),
            None,
        )
        .await;

    let session2 = store
        .get_or_create_session(
            "idempotent-test".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/path/2"),
            None,
        )
        .await;

    let session3 = store
        .get_or_create_session(
            "idempotent-test".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/path/3"),
            None,
        )
        .await;

    assert_eq!(session1.working_dir, PathBuf::from("/path/1"));
    assert_eq!(session2.working_dir, PathBuf::from("/path/1"));
    assert_eq!(session3.working_dir, PathBuf::from("/path/1"));

    let sessions = store.list_all().await;
    assert_eq!(sessions.len(), 1);
}
