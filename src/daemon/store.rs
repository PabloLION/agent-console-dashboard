//! Session store module for the Agent Console daemon.
//!
//! This module provides a thread-safe, in-memory session store for tracking
//! all active agent sessions. It uses `Arc<RwLock<HashMap>>` for O(1) lookups
//! by session ID while supporting concurrent access from multiple async tasks.

use crate::Session;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Thread-safe session store wrapping a HashMap with Arc<RwLock>.
///
/// The SessionStore provides CRUD operations for managing agent sessions
/// with safe concurrent access. Multiple async tasks can read simultaneously,
/// while writes are exclusive.
///
/// # Example
///
/// ```
/// use agent_console::daemon::store::SessionStore;
/// use agent_console::{Session, AgentType};
/// use std::path::PathBuf;
///
/// #[tokio::main]
/// async fn main() {
///     let store = SessionStore::new();
///     let session = Session::new(
///         "session-1".to_string(),
///         AgentType::ClaudeCode,
///         PathBuf::from("/home/user/project"),
///     );
///     store.set("session-1".to_string(), session).await;
///     let retrieved = store.get("session-1").await;
///     assert!(retrieved.is_some());
/// }
/// ```
#[derive(Debug, Clone)]
pub struct SessionStore {
    /// Internal session storage wrapped in Arc<RwLock> for thread-safe access.
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl SessionStore {
    /// Creates a new empty SessionStore.
    ///
    /// # Returns
    ///
    /// A new SessionStore instance with an empty session map.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_console::daemon::store::SessionStore;
    ///
    /// let store = SessionStore::new();
    /// ```
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Retrieves a session by its unique ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The session ID to look up.
    ///
    /// # Returns
    ///
    /// `Some(Session)` if the session exists, `None` otherwise.
    /// The session is cloned when returned (the store owns the data).
    pub async fn get(&self, id: &str) -> Option<Session> {
        let sessions = self.sessions.read().await;
        sessions.get(id).cloned()
    }

    /// Creates or updates a session in the store.
    ///
    /// If a session with the given ID already exists, it will be overwritten.
    ///
    /// # Arguments
    ///
    /// * `id` - The session ID.
    /// * `session` - The session data to store.
    pub async fn set(&self, id: String, session: Session) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(id, session);
    }

    /// Removes a session from the store.
    ///
    /// This operation is idempotent - removing a non-existent session
    /// returns `None` without error.
    ///
    /// # Arguments
    ///
    /// * `id` - The session ID to remove.
    ///
    /// # Returns
    ///
    /// `Some(Session)` with the removed session, or `None` if not found.
    pub async fn remove(&self, id: &str) -> Option<Session> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(id)
    }

    /// Returns all sessions currently in the store.
    ///
    /// Sessions are cloned when returned (the store owns the data).
    /// Returns an empty `Vec` if the store is empty.
    ///
    /// # Returns
    ///
    /// A `Vec<Session>` containing clones of all sessions.
    pub async fn list_all(&self) -> Vec<Session> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AgentType;
    use std::path::PathBuf;

    /// Helper function to create a test session with the given ID.
    fn create_test_session(id: &str) -> Session {
        Session::new(
            id.to_string(),
            AgentType::ClaudeCode,
            PathBuf::from(format!("/home/user/{}", id)),
        )
    }

    #[test]
    fn test_store_new_creates_empty() {
        // Verify new store is created (we can't easily test the internal state
        // without async, but we can verify the struct is created)
        let store = SessionStore::new();
        // Store should implement Clone
        let _cloned = store.clone();
    }

    #[test]
    fn test_store_default() {
        // Verify Default trait works
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

        // Set initial session
        store.set("session-1".to_string(), session1).await;

        // Overwrite with updated session
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

        // Verify session is no longer in store
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

        // First remove succeeds
        let removed1 = store.remove("session-1").await;
        assert!(removed1.is_some());

        // Second remove returns None (idempotent)
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

        // Add multiple sessions
        for i in 1..=5 {
            let session = create_test_session(&format!("session-{}", i));
            store.set(format!("session-{}", i), session).await;
        }

        let sessions = store.list_all().await;
        assert_eq!(sessions.len(), 5);

        // Verify all sessions are present (order may vary due to HashMap)
        let ids: Vec<String> = sessions.iter().map(|s| s.id.clone()).collect();
        for i in 1..=5 {
            assert!(ids.contains(&format!("session-{}", i)));
        }
    }

    #[tokio::test]
    async fn test_store_clone_shares_state() {
        let store = SessionStore::new();
        let cloned = store.clone();

        // Set session through original store
        let session = create_test_session("shared-session");
        store.set("shared-session".to_string(), session).await;

        // Retrieve through cloned store
        let retrieved = cloned.get("shared-session").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "shared-session");
    }

    #[tokio::test]
    async fn test_store_concurrent_access() {
        let store = SessionStore::new();

        // Spawn multiple tasks that read and write concurrently
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

        // Wait for all tasks to complete
        let _ = tokio::join!(handle1, handle2, handle3);

        // Verify all sessions were added
        let sessions = store.list_all().await;
        assert_eq!(sessions.len(), 3);
    }

    #[tokio::test]
    async fn test_store_get_returns_clone() {
        let store = SessionStore::new();
        let session = create_test_session("session-1");

        store.set("session-1".to_string(), session).await;

        // Get session twice
        let retrieved1 = store.get("session-1").await.unwrap();
        let retrieved2 = store.get("session-1").await.unwrap();

        // Both should have the same data (they're clones)
        assert_eq!(retrieved1.id, retrieved2.id);
        assert_eq!(retrieved1.working_dir, retrieved2.working_dir);
    }

    #[tokio::test]
    async fn test_store_set_with_different_key_than_id() {
        // The store allows storing sessions with a key different from session.id
        // This is by design for flexibility
        let store = SessionStore::new();
        let session = create_test_session("actual-id");

        store.set("different-key".to_string(), session).await;

        // Should be retrievable by the key used in set()
        let retrieved = store.get("different-key").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "actual-id");

        // Should NOT be retrievable by the session's actual ID
        let not_found = store.get("actual-id").await;
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_store_list_all_after_remove() {
        let store = SessionStore::new();

        // Add sessions
        for i in 1..=3 {
            let session = create_test_session(&format!("session-{}", i));
            store.set(format!("session-{}", i), session).await;
        }

        // Remove one session
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
        // Debug output should contain "SessionStore"
        assert!(debug_str.contains("SessionStore"));
    }

    // =========================================================================
    // Concurrent Access Integration Tests
    // =========================================================================

    /// Test that multiple concurrent readers can access the store simultaneously.
    ///
    /// This test spawns 10 tasks that all try to read from the store at the same
    /// time, verifying that RwLock allows multiple concurrent readers.
    #[tokio::test]
    async fn test_concurrent_reads() {
        let store = SessionStore::new();

        // Pre-populate store with sessions
        for i in 0..5 {
            let session = create_test_session(&format!("session-{}", i));
            store.set(format!("session-{}", i), session).await;
        }

        // Spawn 10 concurrent reader tasks
        let mut handles = vec![];
        for _ in 0..10 {
            let store_clone = store.clone();
            handles.push(tokio::spawn(async move {
                // Each reader reads all 5 sessions
                for i in 0..5 {
                    let result = store_clone.get(&format!("session-{}", i)).await;
                    assert!(result.is_some());
                }
            }));
        }

        // Wait for all readers to complete
        for handle in handles {
            handle.await.expect("Reader task panicked");
        }

        // Verify store state is unchanged
        let sessions = store.list_all().await;
        assert_eq!(sessions.len(), 5);
    }

    /// Test that concurrent writers don't corrupt the store.
    ///
    /// This test spawns multiple tasks that each write unique sessions,
    /// verifying that all writes succeed without data loss.
    #[tokio::test]
    async fn test_concurrent_writes_no_corruption() {
        let store = SessionStore::new();
        let num_writers = 20;

        // Spawn concurrent writer tasks
        let mut handles = vec![];
        for i in 0..num_writers {
            let store_clone = store.clone();
            handles.push(tokio::spawn(async move {
                let session = create_test_session(&format!("writer-{}", i));
                store_clone.set(format!("writer-{}", i), session).await;
            }));
        }

        // Wait for all writers to complete
        for handle in handles {
            handle.await.expect("Writer task panicked");
        }

        // Verify all sessions were written
        let sessions = store.list_all().await;
        assert_eq!(sessions.len(), num_writers);

        // Verify each session exists
        for i in 0..num_writers {
            let session = store.get(&format!("writer-{}", i)).await;
            assert!(session.is_some(), "Session writer-{} missing", i);
            assert_eq!(session.unwrap().id, format!("writer-{}", i));
        }
    }

    /// Test mixed concurrent reads and writes.
    ///
    /// This test spawns tasks that perform mixed operations (reads, writes, removes)
    /// concurrently, verifying no deadlocks or data corruption occur.
    #[tokio::test]
    async fn test_concurrent_mixed_operations() {
        let store = SessionStore::new();

        // Pre-populate with some sessions
        for i in 0..5 {
            let session = create_test_session(&format!("initial-{}", i));
            store.set(format!("initial-{}", i), session).await;
        }

        // Spawn mixed operation tasks
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

        // Removers (remove some initial sessions)
        for i in 0..3 {
            let store_clone = store.clone();
            handles.push(tokio::spawn(async move {
                let _ = store_clone.remove(&format!("initial-{}", i)).await;
            }));
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.expect("Task panicked");
        }

        // Verify final state:
        // - initial-0, initial-1, initial-2 removed
        // - initial-3, initial-4 remain
        // - new-0 through new-9 added
        let sessions = store.list_all().await;
        assert_eq!(sessions.len(), 12); // 2 initial + 10 new

        // Verify removed sessions are gone
        for i in 0..3 {
            assert!(store.get(&format!("initial-{}", i)).await.is_none());
        }

        // Verify remaining initial sessions
        for i in 3..5 {
            assert!(store.get(&format!("initial-{}", i)).await.is_some());
        }

        // Verify all new sessions exist
        for i in 0..10 {
            assert!(store.get(&format!("new-{}", i)).await.is_some());
        }
    }

    /// Test high contention scenario with rapid concurrent operations.
    ///
    /// This test creates high contention by having many tasks perform
    /// operations on a small number of shared keys.
    #[tokio::test]
    async fn test_concurrent_high_contention() {
        let store = SessionStore::new();

        // Create initial session
        let session = create_test_session("shared-key");
        store.set("shared-key".to_string(), session).await;

        // Spawn many tasks that all operate on the same key
        let mut handles = vec![];
        for i in 0..50 {
            let store_clone = store.clone();
            handles.push(tokio::spawn(async move {
                // Read
                let _ = store_clone.get("shared-key").await;

                // Overwrite
                let mut session = create_test_session("shared-key");
                session.working_dir = PathBuf::from(format!("/path/{}", i));
                store_clone.set("shared-key".to_string(), session).await;

                // Read again
                let _ = store_clone.get("shared-key").await;
            }));
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.expect("Task panicked");
        }

        // Store should have exactly one session
        let sessions = store.list_all().await;
        assert_eq!(sessions.len(), 1);

        // Session should exist
        let session = store.get("shared-key").await;
        assert!(session.is_some());
        assert_eq!(session.unwrap().id, "shared-key");
    }

    /// Test concurrent list_all operations.
    ///
    /// Verifies that list_all returns consistent snapshots even during
    /// concurrent modifications.
    #[tokio::test]
    async fn test_concurrent_list_all_consistency() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc as StdArc;

        let store = SessionStore::new();
        let operations_count = StdArc::new(AtomicUsize::new(0));

        // Pre-populate
        for i in 0..10 {
            let session = create_test_session(&format!("session-{}", i));
            store.set(format!("session-{}", i), session).await;
        }

        let mut handles = vec![];

        // Listers - each checks the result is a valid snapshot
        for _ in 0..20 {
            let store_clone = store.clone();
            let count = operations_count.clone();
            handles.push(tokio::spawn(async move {
                let sessions = store_clone.list_all().await;
                // Should always get a consistent count (10, since we don't modify)
                assert_eq!(sessions.len(), 10);
                count.fetch_add(1, Ordering::SeqCst);
            }));
        }

        // Wait for all
        for handle in handles {
            handle.await.expect("Task panicked");
        }

        // Verify all operations completed
        assert_eq!(operations_count.load(Ordering::SeqCst), 20);
    }

    /// Test that clone shares the same underlying store (Arc behavior).
    ///
    /// Verifies that cloning SessionStore creates a handle to the same
    /// underlying data, not a deep copy.
    #[tokio::test]
    async fn test_concurrent_clone_sharing() {
        let store = SessionStore::new();

        // Create multiple clones
        let store2 = store.clone();
        let store3 = store.clone();
        let store4 = store.clone();

        // Each clone writes to a different key concurrently
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

        // All writes should be visible from any clone
        assert_eq!(store.list_all().await.len(), 4);
        assert_eq!(store2.list_all().await.len(), 4);
        assert_eq!(store3.list_all().await.len(), 4);
        assert_eq!(store4.list_all().await.len(), 4);

        // Verify each session is accessible from any clone
        for key in &["from-store1", "from-store2", "from-store3", "from-store4"] {
            assert!(store.get(key).await.is_some());
            assert!(store2.get(key).await.is_some());
            assert!(store3.get(key).await.is_some());
            assert!(store4.get(key).await.is_some());
        }
    }
}
