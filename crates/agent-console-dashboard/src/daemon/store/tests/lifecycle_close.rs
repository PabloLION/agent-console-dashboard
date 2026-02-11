//! Tests for SessionStore::close_session and remove_session methods.

use super::SessionStore;
use crate::{AgentType, Status};
use std::path::PathBuf;

// =============================================================================
// close_session tests
// =============================================================================

#[tokio::test]
async fn test_close_session() {
    let store = SessionStore::new();

    let _ = store
        .create_session(
            "close-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/home/user/project")),
            None,
        )
        .await;

    let closed = store.close_session("close-test").await;

    assert!(closed.is_some());
    let session = closed.expect("already checked is_some");
    assert_eq!(session.session_id, "close-test");
    assert!(session.closed);
    assert_eq!(session.status, Status::Closed);
}

#[tokio::test]
async fn test_close_session_not_found() {
    let store = SessionStore::new();

    let result = store.close_session("nonexistent").await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_close_session_persists_in_store() {
    let store = SessionStore::new();

    let _ = store
        .create_session(
            "persist-close".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/test")),
            None,
        )
        .await;

    let _ = store.close_session("persist-close").await;

    let retrieved = store.get("persist-close").await;
    assert!(retrieved.is_some());
    let session = retrieved.expect("already checked is_some");
    assert!(session.closed);
    assert_eq!(session.status, Status::Closed);
}

#[tokio::test]
async fn test_close_session_records_transition() {
    let store = SessionStore::new();

    let _ = store
        .create_session(
            "transition-close".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/test")),
            None,
        )
        .await;

    let closed = store.close_session("transition-close").await;

    assert!(closed.is_some());
    let session = closed.expect("already checked is_some");
    assert_eq!(session.history.len(), 1);
    assert_eq!(session.history[0].from, Status::Working);
    assert_eq!(session.history[0].to, Status::Closed);
}

#[tokio::test]
async fn test_close_session_preserves_metadata() {
    let store = SessionStore::new();

    let _ = store
        .create_session(
            "preserve-close".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/specific/path")),
            Some("claude-session-xyz".to_string()),
        )
        .await;

    let closed = store.close_session("preserve-close").await;

    assert!(closed.is_some());
    let session = closed.expect("already checked is_some");

    assert_eq!(session.session_id, "preserve-close");
    assert_eq!(session.agent_type, AgentType::ClaudeCode);
    assert_eq!(session.working_dir, Some(PathBuf::from("/specific/path")));
    assert!(session.closed);
    assert_eq!(session.status, Status::Closed);
}

#[tokio::test]
async fn test_close_session_idempotent() {
    let store = SessionStore::new();

    let _ = store
        .create_session(
            "idempotent-close".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/test")),
            None,
        )
        .await;

    let closed1 = store.close_session("idempotent-close").await;
    let closed2 = store.close_session("idempotent-close").await;

    assert!(closed1.is_some());
    assert!(closed2.is_some());

    let session = closed2.expect("already checked is_some");
    assert!(session.closed);
    assert_eq!(session.status, Status::Closed);
    assert_eq!(session.history.len(), 1);
}

#[tokio::test]
async fn test_close_session_list_all_includes_closed() {
    let store = SessionStore::new();

    let _ = store
        .create_session(
            "session-1".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/path/1")),
            None,
        )
        .await;
    let _ = store
        .create_session(
            "session-2".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/path/2")),
            None,
        )
        .await;

    let _ = store.close_session("session-1").await;

    let sessions = store.list_all().await;
    assert_eq!(sessions.len(), 2);

    let closed_session = sessions.iter().find(|s| s.session_id == "session-1");
    assert!(closed_session.is_some());
    assert!(closed_session.expect("already checked is_some").closed);

    let active_session = sessions.iter().find(|s| s.session_id == "session-2");
    assert!(active_session.is_some());
    assert!(!active_session.expect("already checked is_some").closed);
}

// =============================================================================
// remove_session tests
// =============================================================================

#[tokio::test]
async fn test_remove_session() {
    let store = SessionStore::new();

    let _ = store
        .create_session(
            "session-to-remove".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/path/remove")),
            None,
        )
        .await;
    let _ = store
        .create_session(
            "session-to-close".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/path/close")),
            None,
        )
        .await;
    let _ = store
        .create_session(
            "session-active".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/path/active")),
            None,
        )
        .await;

    let closed = store.close_session("session-to-close").await;
    assert!(closed.is_some());
    assert!(closed.expect("already checked is_some").closed);

    let removed = store.remove_session("session-to-remove").await;
    assert!(removed.is_some());
    assert_eq!(
        removed.expect("already checked is_some").session_id,
        "session-to-remove"
    );

    let sessions = store.list_all().await;
    assert_eq!(sessions.len(), 2);

    let closed_session = sessions.iter().find(|s| s.session_id == "session-to-close");
    assert!(closed_session.is_some());
    let closed_session = closed_session.expect("already checked is_some");
    assert!(closed_session.closed);
    assert_eq!(closed_session.status, Status::Closed);

    let active_session = sessions.iter().find(|s| s.session_id == "session-active");
    assert!(active_session.is_some());
    assert!(!active_session.expect("already checked is_some").closed);

    let removed_session = sessions
        .iter()
        .find(|s| s.session_id == "session-to-remove");
    assert!(removed_session.is_none());

    assert!(store.get("session-to-remove").await.is_none());
    assert!(store.get("session-to-close").await.is_some());
}

#[tokio::test]
async fn test_remove_session_not_found() {
    let store = SessionStore::new();

    let result = store.remove_session("nonexistent").await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_remove_session_idempotent() {
    let store = SessionStore::new();

    let _ = store
        .create_session(
            "to-remove".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/test")),
            None,
        )
        .await;

    let removed1 = store.remove_session("to-remove").await;
    assert!(removed1.is_some());

    let removed2 = store.remove_session("to-remove").await;
    assert!(removed2.is_none());
}

#[tokio::test]
async fn test_remove_session_preserves_data() {
    let store = SessionStore::new();

    let _ = store
        .create_session(
            "data-session".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/specific/path")),
            Some("claude-session-xyz".to_string()),
        )
        .await;

    let removed = store.remove_session("data-session").await;

    assert!(removed.is_some());
    let session = removed.expect("already checked is_some");
    assert_eq!(session.session_id, "data-session");
    assert_eq!(session.agent_type, AgentType::ClaudeCode);
    assert_eq!(session.working_dir, Some(PathBuf::from("/specific/path")));
}
