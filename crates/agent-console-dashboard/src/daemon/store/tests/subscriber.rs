//! Subscriber channel and notification broadcasting tests for SessionStore.

use super::SessionStore;
use crate::{AgentType, Status};
use std::path::PathBuf;

// =========================================================================
// Subscriber Channel Tests
// =========================================================================

#[test]
fn test_store_new_initializes_subscriber_channel() {
    let store = SessionStore::new();
    // Initially, no subscribers (the receiver from channel creation is dropped)
    assert_eq!(store.subscriber_count(), 0);
}

#[test]
fn test_store_subscribe_returns_receiver() {
    let store = SessionStore::new();
    let _rx = store.subscribe();
    // After subscribing, we should have one subscriber
    assert_eq!(store.subscriber_count(), 1);
}

#[test]
fn test_store_multiple_subscribers() {
    let store = SessionStore::new();
    let _rx1 = store.subscribe();
    let _rx2 = store.subscribe();
    let _rx3 = store.subscribe();
    // After multiple subscribes, count should match
    assert_eq!(store.subscriber_count(), 3);
}

#[test]
fn test_store_subscriber_dropped_decrements_count() {
    let store = SessionStore::new();
    let rx1 = store.subscribe();
    let rx2 = store.subscribe();
    assert_eq!(store.subscriber_count(), 2);

    // Drop one receiver
    drop(rx1);
    assert_eq!(store.subscriber_count(), 1);

    // Drop the other
    drop(rx2);
    assert_eq!(store.subscriber_count(), 0);
}

#[test]
fn test_store_clones_share_subscriber_channel() {
    let store = SessionStore::new();
    let cloned = store.clone();

    // Subscribe through original
    let _rx1 = store.subscribe();
    assert_eq!(store.subscriber_count(), 1);
    assert_eq!(cloned.subscriber_count(), 1); // Clone sees same count

    // Subscribe through clone
    let _rx2 = cloned.subscribe();
    assert_eq!(store.subscriber_count(), 2);
    assert_eq!(cloned.subscriber_count(), 2); // Both see same count
}

#[test]
fn test_store_debug_includes_subscriber_count() {
    let store = SessionStore::new();
    let debug_str = format!("{:?}", store);
    // Debug output should contain "SessionStore" and subscriber count info
    assert!(debug_str.contains("SessionStore"));
    assert!(debug_str.contains("subscriber_count"));
}

// =========================================================================
// Subscriber Notification Broadcasting Tests
// =========================================================================

#[tokio::test]
async fn test_subscriber_receives_update_on_status_change() {
    let store = SessionStore::new();
    let mut rx = store.subscribe();

    // Create a session first
    let _ = store
        .create_session(
            "notify-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/home/user/project")),
            None,
        )
        .await;

    // Update the session status
    let _ = store.update_session("notify-test", Status::Attention).await;

    // Subscriber should receive the notification
    let update = rx.try_recv();
    assert!(
        update.is_ok(),
        "Subscriber should receive update notification"
    );
    let update = update.expect("already checked is_ok");
    assert_eq!(update.session_id, "notify-test");
    assert_eq!(update.status, Status::Attention);
}

#[tokio::test]
async fn test_subscriber_no_notification_on_same_status() {
    let store = SessionStore::new();
    let mut rx = store.subscribe();

    // Create a session (starts with Working status)
    let _ = store
        .create_session(
            "same-status-notify".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/test")),
            None,
        )
        .await;

    // Update to the same status (Working)
    let _ = store
        .update_session("same-status-notify", Status::Working)
        .await;

    // Subscriber should NOT receive a notification
    let update = rx.try_recv();
    assert!(
        update.is_err(),
        "Subscriber should not receive notification when status unchanged"
    );
}

#[tokio::test]
async fn test_subscriber_receives_notification_on_close() {
    let store = SessionStore::new();
    let mut rx = store.subscribe();

    // Create a session
    let _ = store
        .create_session(
            "close-notify".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/test")),
            None,
        )
        .await;

    // Close the session
    let _ = store.close_session("close-notify").await;

    // Subscriber should receive the notification
    let update = rx.try_recv();
    assert!(
        update.is_ok(),
        "Subscriber should receive notification on close"
    );
    let update = update.expect("already checked is_ok");
    assert_eq!(update.session_id, "close-notify");
    assert_eq!(update.status, Status::Closed);
}

#[tokio::test]
async fn test_subscriber_no_notification_on_already_closed() {
    let store = SessionStore::new();

    // Create and close a session
    let _ = store
        .create_session(
            "already-closed".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/test")),
            None,
        )
        .await;
    let _ = store.close_session("already-closed").await;

    // Subscribe after first close
    let mut rx = store.subscribe();

    // Try to close again
    let _ = store.close_session("already-closed").await;

    // Subscriber should NOT receive a notification (already closed)
    let update = rx.try_recv();
    assert!(
        update.is_err(),
        "Subscriber should not receive notification when already closed"
    );
}

#[tokio::test]
async fn test_subscriber_multiple_updates_receive_all() {
    let store = SessionStore::new();
    let mut rx = store.subscribe();

    // Create a session
    let _ = store
        .create_session(
            "multi-update".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/test")),
            None,
        )
        .await;

    // Perform multiple status transitions
    let _ = store
        .update_session("multi-update", Status::Attention)
        .await;
    let _ = store.update_session("multi-update", Status::Question).await;
    let _ = store.close_session("multi-update").await;

    // Subscriber should receive all three notifications
    let update1 = rx.try_recv();
    assert!(update1.is_ok());
    assert_eq!(
        update1.expect("already checked is_ok").status,
        Status::Attention
    );

    let update2 = rx.try_recv();
    assert!(update2.is_ok());
    assert_eq!(
        update2.expect("already checked is_ok").status,
        Status::Question
    );

    let update3 = rx.try_recv();
    assert!(update3.is_ok());
    assert_eq!(
        update3.expect("already checked is_ok").status,
        Status::Closed
    );
}

#[tokio::test]
async fn test_subscriber_multiple_subscribers_all_notified() {
    let store = SessionStore::new();
    let mut rx1 = store.subscribe();
    let mut rx2 = store.subscribe();
    let mut rx3 = store.subscribe();

    // Create a session
    let _ = store
        .create_session(
            "multi-subscriber".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/test")),
            None,
        )
        .await;

    // Update session status
    let _ = store
        .update_session("multi-subscriber", Status::Attention)
        .await;

    // All subscribers should receive the notification
    let update1 = rx1.try_recv();
    let update2 = rx2.try_recv();
    let update3 = rx3.try_recv();

    assert!(update1.is_ok(), "Subscriber 1 should receive notification");
    assert!(update2.is_ok(), "Subscriber 2 should receive notification");
    assert!(update3.is_ok(), "Subscriber 3 should receive notification");

    // All should have the same content
    assert_eq!(
        update1.expect("already checked is_ok").status,
        Status::Attention
    );
    assert_eq!(
        update2.expect("already checked is_ok").status,
        Status::Attention
    );
    assert_eq!(
        update3.expect("already checked is_ok").status,
        Status::Attention
    );
}

#[tokio::test]
async fn test_subscriber_notification_does_not_block_without_subscribers() {
    let store = SessionStore::new();
    // No subscribers

    // Create a session
    let _ = store
        .create_session(
            "no-subscriber".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/test")),
            None,
        )
        .await;

    // Update session status - should not block or panic even without subscribers
    let result = store
        .update_session("no-subscriber", Status::Attention)
        .await;
    assert!(result.is_some());

    // Close session - should also work
    let closed = store.close_session("no-subscriber").await;
    assert!(closed.is_some());
}

#[tokio::test]
async fn test_subscriber_update_contains_correct_elapsed_seconds() {
    use std::time::Duration;

    let store = SessionStore::new();
    let mut rx = store.subscribe();

    // Create a session
    let _ = store
        .create_session(
            "elapsed-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/test")),
            None,
        )
        .await;

    // Wait a short time
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Update session status
    let _ = store
        .update_session("elapsed-test", Status::Attention)
        .await;

    // Subscriber should receive notification with elapsed_seconds >= 0
    let update = rx.try_recv();
    assert!(update.is_ok());
    let update = update.expect("already checked is_ok");
    // Elapsed seconds should be 0 or small (since we just changed status)
    // The elapsed is calculated from the updated 'since' timestamp,
    // so right after status change it should be very small
    assert!(
        update.elapsed_seconds < 5,
        "Elapsed seconds should be small right after status change"
    );
}
