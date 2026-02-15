//! Tests for closed session storage in SessionStore.

use super::SessionStore;
use crate::{AgentType, Status};
use std::path::PathBuf;

// =============================================================================
// close_session stores ClosedSession metadata
// =============================================================================

#[tokio::test]
async fn close_session_stores_closed_metadata() {
    let store = SessionStore::new();

    let _ = store
        .create_session(
            "s1".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/project")),
            Some("claude-abc".to_string()),
        )
        .await;

    let _ = store.close_session("s1").await;

    let closed = store.list_closed().await;
    assert_eq!(closed.len(), 1);

    let meta = &closed[0];
    assert_eq!(meta.session_id, "s1");
    assert_eq!(meta.working_dir, Some(PathBuf::from("/tmp/project")));
    assert!(meta.resumable); // Sessions with working_dir are resumable
    assert!(meta.not_resumable_reason.is_none());
    assert_eq!(meta.last_status, Status::Closed);
}

#[tokio::test]
async fn close_session_without_working_dir_not_resumable() {
    let store = SessionStore::new();

    // Create a session without working_dir
    let mut session = crate::Session::new(
        "s2".to_string(),
        AgentType::ClaudeCode,
        None, // No working dir
    );
    store.set("s2".to_string(), session).await;

    let _ = store.close_session("s2").await;

    let closed = store.list_closed().await;
    assert_eq!(closed.len(), 1);
    assert!(!closed[0].resumable);
    assert!(closed[0].not_resumable_reason.is_some());
}

// =============================================================================
// list_closed returns most recent first
// =============================================================================

#[tokio::test]
async fn list_closed_returns_most_recent_first() {
    let store = SessionStore::new();

    for i in 1..=3 {
        let id = format!("s{}", i);
        let _ = store
            .create_session(
                id.clone(),
                AgentType::ClaudeCode,
                Some(PathBuf::from(format!("/tmp/{}", i))),
                None,
            )
            .await;
        let _ = store.close_session(&id).await;
    }

    let closed = store.list_closed().await;
    assert_eq!(closed.len(), 3);
    assert_eq!(closed[0].session_id, "s3"); // most recent
    assert_eq!(closed[1].session_id, "s2");
    assert_eq!(closed[2].session_id, "s1"); // oldest
}

// =============================================================================
// get_closed
// =============================================================================

#[tokio::test]
async fn get_closed_returns_correct_session() {
    let store = SessionStore::new();

    let _ = store
        .create_session(
            "target".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/target")),
            Some("resume-id".to_string()),
        )
        .await;
    let _ = store.close_session("target").await;

    let result = store.get_closed("target").await;
    assert!(result.is_some());
    let meta = result.expect("already checked is_some");
    assert_eq!(meta.session_id, "target");
    assert!(meta.resumable); // Sessions with working_dir are resumable
}

#[tokio::test]
async fn get_closed_returns_none_for_unknown() {
    let store = SessionStore::new();
    assert!(store.get_closed("nonexistent").await.is_none());
}

// =============================================================================
// Retention limit
// =============================================================================

#[tokio::test]
async fn retention_limit_evicts_oldest() {
    let store = SessionStore::new();
    // Default limit is 20, create and close 25 sessions
    for i in 1..=25 {
        let id = format!("s{}", i);
        let _ = store
            .create_session(
                id.clone(),
                AgentType::ClaudeCode,
                Some(PathBuf::from(format!("/tmp/{}", i))),
                None,
            )
            .await;
        let _ = store.close_session(&id).await;
    }

    let closed = store.list_closed().await;
    assert_eq!(closed.len(), 20);

    // Oldest 5 (s1..s5) should be evicted
    assert!(store.get_closed("s1").await.is_none());
    assert!(store.get_closed("s5").await.is_none());
    // s6 should still exist
    assert!(store.get_closed("s6").await.is_some());
    // Most recent should be s25
    assert_eq!(closed[0].session_id, "s25");
}

// =============================================================================
// Deduplication
// =============================================================================

#[tokio::test]
async fn closing_same_session_twice_deduplicates() {
    let store = SessionStore::new();

    let _ = store
        .create_session(
            "dup".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/dup")),
            None,
        )
        .await;

    let _ = store.close_session("dup").await;
    let _ = store.close_session("dup").await;

    let closed = store.list_closed().await;
    assert_eq!(closed.len(), 1);
    assert_eq!(closed[0].session_id, "dup");
}

// =============================================================================
// Multiple sessions
// =============================================================================

#[tokio::test]
async fn multiple_sessions_can_be_closed() {
    let store = SessionStore::new();

    for id in ["a", "b", "c"] {
        let _ = store
            .create_session(
                id.to_string(),
                AgentType::ClaudeCode,
                Some(PathBuf::from(format!("/tmp/{}", id))),
                None,
            )
            .await;
        let _ = store.close_session(id).await;
    }

    let closed = store.list_closed().await;
    assert_eq!(closed.len(), 3);
}

// =============================================================================
// Active sessions still work after adding closed storage
// =============================================================================

#[tokio::test]
async fn active_sessions_unaffected_by_closed_storage() {
    let store = SessionStore::new();

    let _ = store
        .create_session(
            "active".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/active")),
            None,
        )
        .await;
    let _ = store
        .create_session(
            "to-close".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/close")),
            None,
        )
        .await;

    let _ = store.close_session("to-close").await;

    // Active session still retrievable and working
    let active = store.get("active").await;
    assert!(active.is_some());
    let session = active.expect("already checked is_some");
    assert_eq!(session.status, Status::Working);
    assert!(!session.closed);

    // Can still update active session
    let updated = store.update_session("active", Status::Attention).await;
    assert!(updated.is_some());
    assert_eq!(
        updated.expect("already checked is_some").status,
        Status::Attention
    );

    // list_all still includes both
    let all = store.list_all().await;
    assert_eq!(all.len(), 2);
}

// =============================================================================
// Closed metadata includes required fields
// =============================================================================

#[tokio::test]
async fn closed_metadata_includes_all_fields() {
    let store = SessionStore::new();

    let _ = store
        .create_session(
            "full".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/home/user/project")),
            Some("claude-session-123".to_string()),
        )
        .await;

    // Transition through statuses before closing
    let _ = store.update_session("full", Status::Attention).await;
    let _ = store.close_session("full").await;

    let meta = store
        .get_closed("full")
        .await
        .expect("closed session should exist");

    assert_eq!(meta.session_id, "full");
    assert_eq!(meta.working_dir, Some(PathBuf::from("/home/user/project")));
    assert!(meta.resumable); // Sessions with working_dir are resumable
    assert!(meta.not_resumable_reason.is_none());
    assert_eq!(meta.last_status, Status::Closed);
    assert!(meta.closed_at.is_some());
    // Elapsed values should be non-negative (they are u64 so always >= 0)
    // Just verify they exist
    let _ = meta.started_at_elapsed;
    let _ = meta.closed_at_elapsed;
}
