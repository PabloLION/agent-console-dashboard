//! Concurrent access and thread-safety tests for SessionStore.

use super::{create_test_session, SessionStore};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc as StdArc;

/// Test that multiple concurrent readers can access the store simultaneously.
#[tokio::test]
async fn test_concurrent_reads() {
    let store = SessionStore::new();

    for i in 0..5 {
        let session = create_test_session(&format!("session-{}", i));
        store.set(format!("session-{}", i), session).await;
    }

    let mut handles = vec![];
    for _ in 0..10 {
        let store_clone = store.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..5 {
                let result = store_clone.get(&format!("session-{}", i)).await;
                assert!(result.is_some());
            }
        }));
    }

    for handle in handles {
        handle.await.expect("Reader task panicked");
    }

    let sessions = store.list_all().await;
    assert_eq!(sessions.len(), 5);
}

/// Test that concurrent writers don't corrupt the store.
#[tokio::test]
async fn test_concurrent_writes_no_corruption() {
    let store = SessionStore::new();
    let num_writers = 20;

    let mut handles = vec![];
    for i in 0..num_writers {
        let store_clone = store.clone();
        handles.push(tokio::spawn(async move {
            let session = create_test_session(&format!("writer-{}", i));
            store_clone.set(format!("writer-{}", i), session).await;
        }));
    }

    for handle in handles {
        handle.await.expect("Writer task panicked");
    }

    let sessions = store.list_all().await;
    assert_eq!(sessions.len(), num_writers);

    for i in 0..num_writers {
        let session = store.get(&format!("writer-{}", i)).await;
        assert!(session.is_some(), "Session writer-{} missing", i);
        assert_eq!(session.unwrap().id, format!("writer-{}", i));
    }
}

/// Test mixed concurrent reads and writes.
#[tokio::test]
async fn test_concurrent_mixed_operations() {
    let store = SessionStore::new();

    for i in 0..5 {
        let session = create_test_session(&format!("initial-{}", i));
        store.set(format!("initial-{}", i), session).await;
    }

    let mut handles = vec![];

    // Writers
    for i in 0..10 {
        let store_clone = store.clone();
        handles.push(tokio::spawn(async move {
            let session = create_test_session(&format!("new-{}", i));
            store_clone.set(format!("new-{}", i), session).await;
        }));
    }

    // Readers
    for _ in 0..10 {
        let store_clone = store.clone();
        handles.push(tokio::spawn(async move {
            let _ = store_clone.list_all().await;
        }));
    }

    // Removers
    for i in 0..3 {
        let store_clone = store.clone();
        handles.push(tokio::spawn(async move {
            let _ = store_clone.remove(&format!("initial-{}", i)).await;
        }));
    }

    for handle in handles {
        handle.await.expect("Task panicked");
    }

    let sessions = store.list_all().await;
    assert_eq!(sessions.len(), 12); // 2 initial + 10 new

    for i in 0..3 {
        assert!(store.get(&format!("initial-{}", i)).await.is_none());
    }

    for i in 3..5 {
        assert!(store.get(&format!("initial-{}", i)).await.is_some());
    }

    for i in 0..10 {
        assert!(store.get(&format!("new-{}", i)).await.is_some());
    }
}

/// Test high contention scenario with rapid concurrent operations.
#[tokio::test]
async fn test_concurrent_high_contention() {
    let store = SessionStore::new();

    let session = create_test_session("shared-key");
    store.set("shared-key".to_string(), session).await;

    let mut handles = vec![];
    for i in 0..50 {
        let store_clone = store.clone();
        handles.push(tokio::spawn(async move {
            let _ = store_clone.get("shared-key").await;

            let mut session = create_test_session("shared-key");
            session.working_dir = PathBuf::from(format!("/path/{}", i));
            store_clone.set("shared-key".to_string(), session).await;

            let _ = store_clone.get("shared-key").await;
        }));
    }

    for handle in handles {
        handle.await.expect("Task panicked");
    }

    let sessions = store.list_all().await;
    assert_eq!(sessions.len(), 1);

    let session = store.get("shared-key").await;
    assert!(session.is_some());
    assert_eq!(session.unwrap().id, "shared-key");
}

/// Test concurrent list_all operations return consistent snapshots.
#[tokio::test]
async fn test_concurrent_list_all_consistency() {
    let store = SessionStore::new();
    let operations_count = StdArc::new(AtomicUsize::new(0));

    for i in 0..10 {
        let session = create_test_session(&format!("session-{}", i));
        store.set(format!("session-{}", i), session).await;
    }

    let mut handles = vec![];

    for _ in 0..20 {
        let store_clone = store.clone();
        let count = operations_count.clone();
        handles.push(tokio::spawn(async move {
            let sessions = store_clone.list_all().await;
            assert_eq!(sessions.len(), 10);
            count.fetch_add(1, Ordering::SeqCst);
        }));
    }

    for handle in handles {
        handle.await.expect("Task panicked");
    }

    assert_eq!(operations_count.load(Ordering::SeqCst), 20);
}

/// Test that clone shares the same underlying store (Arc behavior).
#[tokio::test]
async fn test_concurrent_clone_sharing() {
    let store = SessionStore::new();

    let store2 = store.clone();
    let store3 = store.clone();
    let store4 = store.clone();

    let handles = vec![
        tokio::spawn({
            let s = store.clone();
            async move {
                s.set(
                    "from-store1".to_string(),
                    create_test_session("from-store1"),
                )
                .await;
            }
        }),
        tokio::spawn({
            let s = store2.clone();
            async move {
                s.set(
                    "from-store2".to_string(),
                    create_test_session("from-store2"),
                )
                .await;
            }
        }),
        tokio::spawn({
            let s = store3.clone();
            async move {
                s.set(
                    "from-store3".to_string(),
                    create_test_session("from-store3"),
                )
                .await;
            }
        }),
        tokio::spawn({
            let s = store4.clone();
            async move {
                s.set(
                    "from-store4".to_string(),
                    create_test_session("from-store4"),
                )
                .await;
            }
        }),
    ];

    for handle in handles {
        handle.await.expect("Task panicked");
    }

    assert_eq!(store.list_all().await.len(), 4);
    assert_eq!(store2.list_all().await.len(), 4);
    assert_eq!(store3.list_all().await.len(), 4);
    assert_eq!(store4.list_all().await.len(), 4);

    for key in &["from-store1", "from-store2", "from-store3", "from-store4"] {
        assert!(store.get(key).await.is_some());
        assert!(store2.get(key).await.is_some());
        assert!(store3.get(key).await.is_some());
        assert!(store4.get(key).await.is_some());
    }
}

#[tokio::test]
async fn test_store_concurrent_access() {
    let store = SessionStore::new();

    let store1 = store.clone();
    let store2 = store.clone();
    let store3 = store.clone();

    let handle1 = tokio::spawn(async move {
        let session = create_test_session("task-1-session");
        store1.set("task-1-session".to_string(), session).await;
    });

    let handle2 = tokio::spawn(async move {
        let session = create_test_session("task-2-session");
        store2.set("task-2-session".to_string(), session).await;
    });

    let handle3 = tokio::spawn(async move {
        let session = create_test_session("task-3-session");
        store3.set("task-3-session".to_string(), session).await;
    });

    let _ = tokio::join!(handle1, handle2, handle3);

    let sessions = store.list_all().await;
    assert_eq!(sessions.len(), 3);
}
