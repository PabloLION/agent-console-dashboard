//! Basic CRUD operation tests for SessionStore.

use super::{create_test_session, SessionStore};
use crate::AgentType;
use std::path::PathBuf;

#[test]
fn test_store_new_creates_empty() {
    let store = SessionStore::new();
    let _cloned = store.clone();
}

#[test]
fn test_store_default() {
    let store = SessionStore::default();
    let _cloned = store.clone();
}

#[tokio::test]
async fn test_store_get_nonexistent_returns_none() {
    let store = SessionStore::new();
    let result = store.get("nonexistent-id").await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_store_set_and_get() {
    let store = SessionStore::new();
    let session = create_test_session("session-1");

    store.set("session-1".to_string(), session.clone()).await;
    let retrieved = store.get("session-1").await;

    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, "session-1");
    assert_eq!(retrieved.agent_type, AgentType::ClaudeCode);
    assert_eq!(retrieved.working_dir, PathBuf::from("/home/user/session-1"));
}

#[tokio::test]
async fn test_store_set_overwrites_existing() {
    let store = SessionStore::new();
    let session1 = create_test_session("session-1");
    let mut session2 = create_test_session("session-1");
    session2.working_dir = PathBuf::from("/updated/path");

    store.set("session-1".to_string(), session1).await;
    store.set("session-1".to_string(), session2).await;

    let retrieved = store.get("session-1").await.unwrap();
    assert_eq!(retrieved.working_dir, PathBuf::from("/updated/path"));
}

#[tokio::test]
async fn test_store_remove_existing() {
    let store = SessionStore::new();
    let session = create_test_session("session-1");

    store.set("session-1".to_string(), session).await;
    let removed = store.remove("session-1").await;

    assert!(removed.is_some());
    assert_eq!(removed.unwrap().id, "session-1");

    let retrieved = store.get("session-1").await;
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_store_remove_nonexistent_returns_none() {
    let store = SessionStore::new();
    let removed = store.remove("nonexistent-id").await;
    assert!(removed.is_none());
}

#[tokio::test]
async fn test_store_remove_is_idempotent() {
    let store = SessionStore::new();
    let session = create_test_session("session-1");

    store.set("session-1".to_string(), session).await;

    let removed1 = store.remove("session-1").await;
    assert!(removed1.is_some());

    let removed2 = store.remove("session-1").await;
    assert!(removed2.is_none());
}

#[tokio::test]
async fn test_store_list_all_empty() {
    let store = SessionStore::new();
    let sessions = store.list_all().await;
    assert!(sessions.is_empty());
}

#[tokio::test]
async fn test_store_list_all_single_session() {
    let store = SessionStore::new();
    let session = create_test_session("session-1");

    store.set("session-1".to_string(), session).await;
    let sessions = store.list_all().await;

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, "session-1");
}

#[tokio::test]
async fn test_store_list_all_multiple_sessions() {
    let store = SessionStore::new();

    for i in 1..=5 {
        let session = create_test_session(&format!("session-{}", i));
        store.set(format!("session-{}", i), session).await;
    }

    let sessions = store.list_all().await;
    assert_eq!(sessions.len(), 5);

    let ids: Vec<String> = sessions.iter().map(|s| s.id.clone()).collect();
    for i in 1..=5 {
        assert!(ids.contains(&format!("session-{}", i)));
    }
}

#[tokio::test]
async fn test_store_clone_shares_state() {
    let store = SessionStore::new();
    let cloned = store.clone();

    let session = create_test_session("shared-session");
    store.set("shared-session".to_string(), session).await;

    let retrieved = cloned.get("shared-session").await;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, "shared-session");
}

#[tokio::test]
async fn test_store_get_returns_clone() {
    let store = SessionStore::new();
    let session = create_test_session("session-1");

    store.set("session-1".to_string(), session).await;

    let retrieved1 = store.get("session-1").await.unwrap();
    let retrieved2 = store.get("session-1").await.unwrap();

    assert_eq!(retrieved1.id, retrieved2.id);
    assert_eq!(retrieved1.working_dir, retrieved2.working_dir);
}

#[tokio::test]
async fn test_store_set_with_different_key_than_id() {
    let store = SessionStore::new();
    let session = create_test_session("actual-id");

    store.set("different-key".to_string(), session).await;

    let retrieved = store.get("different-key").await;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, "actual-id");

    let not_found = store.get("actual-id").await;
    assert!(not_found.is_none());
}

#[tokio::test]
async fn test_store_list_all_after_remove() {
    let store = SessionStore::new();

    for i in 1..=3 {
        let session = create_test_session(&format!("session-{}", i));
        store.set(format!("session-{}", i), session).await;
    }

    store.remove("session-2").await;

    let sessions = store.list_all().await;
    assert_eq!(sessions.len(), 2);

    let ids: Vec<String> = sessions.iter().map(|s| s.id.clone()).collect();
    assert!(ids.contains(&"session-1".to_string()));
    assert!(ids.contains(&"session-3".to_string()));
    assert!(!ids.contains(&"session-2".to_string()));
}

#[tokio::test]
async fn test_store_debug_format() {
    let store = SessionStore::new();
    let debug_str = format!("{:?}", store);
    assert!(debug_str.contains("SessionStore"));
}
