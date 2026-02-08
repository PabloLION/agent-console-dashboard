//! Tests for inactive session detection (no auto-close).

use super::SessionStore;
use crate::AgentType;
use std::path::PathBuf;
use std::time::Duration;

const THRESHOLD: Duration = Duration::from_secs(3600);

/// Helper: create a session and backdate its `last_activity` via the public API.
async fn create_inactive_session(store: &SessionStore, id: &str, inactive_secs: u64) {
    let _ = store
        .create_session(
            id.to_string(),
            AgentType::ClaudeCode,
            PathBuf::from(format!("/tmp/{id}")),
            None,
        )
        .await;

    let mut session = store.get(id).await.expect("session just created");
    session.last_activity = session
        .last_activity
        .checked_sub(Duration::from_secs(inactive_secs))
        .expect("backdate should succeed");
    store.set(id.to_string(), session).await;
}

// =============================================================================
// count_inactive_sessions
// =============================================================================

#[tokio::test]
async fn counts_inactive_sessions() {
    let store = SessionStore::new();
    create_inactive_session(&store, "old-1", 7200).await;

    let count = store.count_inactive_sessions(THRESHOLD).await;
    assert_eq!(count, 1);

    // Session is NOT closed — still in store with original status
    let session = store.get("old-1").await.expect("session still in store");
    assert!(!session.closed, "inactive sessions should not be auto-closed");
}

#[tokio::test]
async fn fresh_sessions_not_counted() {
    let store = SessionStore::new();
    let _ = store
        .create_session(
            "fresh-1".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/tmp/fresh"),
            None,
        )
        .await;

    let count = store.count_inactive_sessions(THRESHOLD).await;
    assert_eq!(count, 0);
}

#[tokio::test]
async fn mixed_active_and_inactive() {
    let store = SessionStore::new();
    create_inactive_session(&store, "inactive", 7200).await;

    let _ = store
        .create_session(
            "active".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/tmp/active"),
            None,
        )
        .await;

    let count = store.count_inactive_sessions(THRESHOLD).await;
    assert_eq!(count, 1);
}

#[tokio::test]
async fn closed_sessions_not_counted_as_inactive() {
    let store = SessionStore::new();
    create_inactive_session(&store, "closed-old", 7200).await;
    let _ = store.close_session("closed-old").await;

    let count = store.count_inactive_sessions(THRESHOLD).await;
    assert_eq!(count, 0);
}

#[tokio::test]
async fn empty_store_returns_zero() {
    let store = SessionStore::new();
    let count = store.count_inactive_sessions(THRESHOLD).await;
    assert_eq!(count, 0);
}

// =============================================================================
// has_active_sessions excludes inactive
// =============================================================================

#[tokio::test]
async fn has_active_excludes_inactive_sessions() {
    let store = SessionStore::new();
    create_inactive_session(&store, "inactive-only", 7200).await;

    assert!(
        !store.has_active_sessions(THRESHOLD).await,
        "inactive session should not count as active"
    );
}

#[tokio::test]
async fn has_active_includes_fresh_sessions() {
    let store = SessionStore::new();
    let _ = store
        .create_session(
            "fresh".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/tmp/fresh"),
            None,
        )
        .await;

    assert!(store.has_active_sessions(THRESHOLD).await);
}

#[tokio::test]
async fn has_active_mixed_returns_true() {
    let store = SessionStore::new();
    create_inactive_session(&store, "inactive", 7200).await;

    let _ = store
        .create_session(
            "active".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/tmp/active"),
            None,
        )
        .await;

    assert!(
        store.has_active_sessions(THRESHOLD).await,
        "one active session should be enough"
    );
}
