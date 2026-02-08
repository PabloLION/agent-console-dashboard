//! Tests for stale session detection and auto-close.

use super::SessionStore;
use crate::{AgentType, Status};
use std::path::PathBuf;
use std::time::Duration;

/// Helper: create a session and backdate its `last_activity` via the public API.
async fn create_stale_session(store: &SessionStore, id: &str, stale_secs: u64) {
    let _ = store
        .create_session(
            id.to_string(),
            AgentType::ClaudeCode,
            PathBuf::from(format!("/tmp/{id}")),
            None,
        )
        .await;

    // Backdate last_activity by replacing the session with a modified copy
    let mut session = store.get(id).await.expect("session just created");
    session.last_activity = session
        .last_activity
        .checked_sub(Duration::from_secs(stale_secs))
        .expect("backdate should succeed");
    store.set(id.to_string(), session).await;
}

// =============================================================================
// close_stale_sessions — basic behavior
// =============================================================================

#[tokio::test]
async fn closes_old_sessions() {
    let store = SessionStore::new();
    create_stale_session(&store, "stale-1", 7200).await;

    let count = store
        .close_stale_sessions(Duration::from_secs(3600))
        .await;

    assert_eq!(count, 1);
    let session = store.get("stale-1").await.expect("session still in store");
    assert!(session.closed);
    assert_eq!(session.status, Status::Closed);
}

#[tokio::test]
async fn preserves_fresh_sessions() {
    let store = SessionStore::new();
    let _ = store
        .create_session(
            "fresh-1".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/tmp/fresh"),
            None,
        )
        .await;

    let count = store
        .close_stale_sessions(Duration::from_secs(3600))
        .await;

    assert_eq!(count, 0);
    let session = store.get("fresh-1").await.expect("session exists");
    assert!(!session.closed);
}

#[tokio::test]
async fn returns_correct_count() {
    let store = SessionStore::new();
    create_stale_session(&store, "s1", 7200).await;
    create_stale_session(&store, "s2", 7200).await;
    create_stale_session(&store, "s3", 7200).await;

    let count = store
        .close_stale_sessions(Duration::from_secs(3600))
        .await;

    assert_eq!(count, 3);
}

#[tokio::test]
async fn mixed_stale_and_fresh() {
    let store = SessionStore::new();
    create_stale_session(&store, "stale", 7200).await;

    let _ = store
        .create_session(
            "fresh".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/tmp/fresh"),
            None,
        )
        .await;

    let count = store
        .close_stale_sessions(Duration::from_secs(3600))
        .await;

    assert_eq!(count, 1);

    let stale = store.get("stale").await.expect("stale session exists");
    assert!(stale.closed);

    let fresh = store.get("fresh").await.expect("fresh session exists");
    assert!(!fresh.closed);
}

#[tokio::test]
async fn already_closed_sessions_ignored() {
    let store = SessionStore::new();
    create_stale_session(&store, "already-closed", 7200).await;
    let _ = store.close_session("already-closed").await;

    let count = store
        .close_stale_sessions(Duration::from_secs(3600))
        .await;

    assert_eq!(count, 0, "already-closed sessions should not be re-closed");
}

#[tokio::test]
async fn stale_sessions_archived_for_resurrection() {
    let store = SessionStore::new();
    create_stale_session(&store, "archivable", 7200).await;

    let _ = store
        .close_stale_sessions(Duration::from_secs(3600))
        .await;

    let closed_list = store.list_closed().await;
    assert_eq!(closed_list.len(), 1);
    assert_eq!(closed_list[0].session_id, "archivable");
}

#[tokio::test]
async fn no_sessions_returns_zero() {
    let store = SessionStore::new();
    let count = store
        .close_stale_sessions(Duration::from_secs(3600))
        .await;
    assert_eq!(count, 0);
}
