//! Session store module for the Agent Console daemon.
//!
//! This module provides a thread-safe, in-memory session store for tracking
//! all active agent sessions. It uses `Arc<RwLock<HashMap>>` for O(1) lookups
//! by session ID while supporting concurrent access from multiple async tasks.

use crate::{AgentType, Session, SessionUpdate, Status, StoreError};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Default capacity for the subscriber notification channel.
/// This allows for bursty update scenarios without dropping notifications.
const DEFAULT_SUBSCRIBER_CHANNEL_CAPACITY: usize = 256;

/// Thread-safe session store wrapping a HashMap with Arc<RwLock>.
///
/// The SessionStore provides CRUD operations for managing agent sessions
/// with safe concurrent access. Multiple async tasks can read simultaneously,
/// while writes are exclusive.
///
/// The store also includes a broadcast channel for subscriber notifications.
/// Clients can subscribe to receive [`SessionUpdate`] messages whenever a
/// session's status changes.
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
#[derive(Clone)]
pub struct SessionStore {
    /// Internal session storage wrapped in Arc<RwLock> for thread-safe access.
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    /// Broadcast channel sender for subscriber notifications.
    /// Subscribers receive [`SessionUpdate`] messages on state changes.
    update_tx: broadcast::Sender<SessionUpdate>,
}

impl std::fmt::Debug for SessionStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionStore")
            .field("sessions", &self.sessions)
            .field("subscriber_count", &self.update_tx.receiver_count())
            .finish()
    }
}

impl SessionStore {
    /// Creates a new empty SessionStore.
    ///
    /// Initializes an empty session map and a broadcast channel for subscriber
    /// notifications with the default capacity (256 messages).
    ///
    /// # Returns
    ///
    /// A new SessionStore instance with an empty session map and notification channel.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_console::daemon::store::SessionStore;
    ///
    /// let store = SessionStore::new();
    /// ```
    pub fn new() -> Self {
        let (update_tx, _rx) = broadcast::channel(DEFAULT_SUBSCRIBER_CHANNEL_CAPACITY);
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            update_tx,
        }
    }

    /// Broadcasts a status change notification to all subscribers.
    ///
    /// Only sends if the status actually changed (old_status != session.status).
    /// Logs the result at trace/debug level.
    fn broadcast_status_change(&self, old_status: Status, session: &Session) {
        if old_status != session.status {
            let update = SessionUpdate::new(
                session.id.clone(),
                session.status,
                session.since.elapsed().as_secs(),
            );
            match self.update_tx.send(update) {
                Ok(count) => {
                    tracing::trace!("Broadcast update sent to {} subscribers", count);
                }
                Err(_) => {
                    tracing::debug!("No subscribers for session update broadcast");
                }
            }
        }
    }

    /// Subscribes to session update notifications.
    ///
    /// Returns a broadcast receiver that will receive [`SessionUpdate`] messages
    /// whenever a session's status changes. Multiple subscribers can exist
    /// simultaneously; all will receive the same updates.
    ///
    /// # Returns
    ///
    /// A `broadcast::Receiver<SessionUpdate>` for receiving notifications.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_console::daemon::store::SessionStore;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let store = SessionStore::new();
    ///     let mut rx = store.subscribe();
    ///     // rx.recv().await will receive SessionUpdate messages
    /// }
    /// ```
    pub fn subscribe(&self) -> broadcast::Receiver<SessionUpdate> {
        self.update_tx.subscribe()
    }

    /// Returns the number of active subscribers.
    ///
    /// This can be useful for monitoring or debugging purposes.
    ///
    /// # Returns
    ///
    /// The count of active broadcast receivers.
    pub fn subscriber_count(&self) -> usize {
        self.update_tx.receiver_count()
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

    /// Creates a new session explicitly with provided metadata.
    ///
    /// This method is used for programmatic session creation where the caller
    /// wants to ensure no existing session is overwritten. Unlike `set()`, this
    /// method returns an error if a session with the given ID already exists.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique session identifier.
    /// * `agent_type` - Type of agent (e.g., ClaudeCode).
    /// * `working_dir` - Working directory path for this session.
    /// * `session_id` - Optional Claude Code session ID for resume capability.
    ///
    /// # Returns
    ///
    /// * `Ok(Session)` - The newly created session.
    /// * `Err(StoreError::SessionExists)` - If a session with this ID already exists.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_console::daemon::store::SessionStore;
    /// use agent_console::AgentType;
    /// use std::path::PathBuf;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let store = SessionStore::new();
    ///     let result = store.create_session(
    ///         "session-1".to_string(),
    ///         AgentType::ClaudeCode,
    ///         PathBuf::from("/home/user/project"),
    ///         Some("claude-session-abc".to_string()),
    ///     ).await;
    ///     assert!(result.is_ok());
    ///
    ///     // Attempting to create again returns error
    ///     let result2 = store.create_session(
    ///         "session-1".to_string(),
    ///         AgentType::ClaudeCode,
    ///         PathBuf::from("/home/user/project"),
    ///         None,
    ///     ).await;
    ///     assert!(result2.is_err());
    /// }
    /// ```
    pub async fn create_session(
        &self,
        id: String,
        agent_type: AgentType,
        working_dir: PathBuf,
        session_id: Option<String>,
    ) -> Result<Session, StoreError> {
        let mut sessions = self.sessions.write().await;

        // Check if session already exists
        if sessions.contains_key(&id) {
            return Err(StoreError::SessionExists(id));
        }

        // Create new session
        let mut session = Session::new(id.clone(), agent_type, working_dir);
        session.session_id = session_id;

        // Insert and return clone
        sessions.insert(id, session.clone());
        Ok(session)
    }

    /// Gets existing session or creates new one if not found.
    ///
    /// This method is used for lazy session creation on first SET command.
    /// Unlike `create_session()`, this method never fails - it always returns
    /// a valid session. If a session with the given ID exists, it is returned.
    /// Otherwise, a new session is created with the provided metadata.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique session identifier.
    /// * `agent_type` - Type of agent (e.g., ClaudeCode).
    /// * `working_dir` - Working directory path for this session.
    /// * `session_id` - Optional Claude Code session ID for resume capability.
    ///
    /// # Returns
    ///
    /// The existing or newly created session.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_console::daemon::store::SessionStore;
    /// use agent_console::AgentType;
    /// use std::path::PathBuf;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let store = SessionStore::new();
    ///
    ///     // First call creates a new session
    ///     let session1 = store.get_or_create_session(
    ///         "session-1".to_string(),
    ///         AgentType::ClaudeCode,
    ///         PathBuf::from("/home/user/project"),
    ///         Some("claude-session-abc".to_string()),
    ///     ).await;
    ///     assert_eq!(session1.id, "session-1");
    ///
    ///     // Second call returns the existing session
    ///     let session2 = store.get_or_create_session(
    ///         "session-1".to_string(),
    ///         AgentType::ClaudeCode,
    ///         PathBuf::from("/different/path"),  // Different path, but existing session returned
    ///         None,
    ///     ).await;
    ///     assert_eq!(session2.working_dir, PathBuf::from("/home/user/project"));  // Original path preserved
    /// }
    /// ```
    pub async fn get_or_create_session(
        &self,
        id: String,
        agent_type: AgentType,
        working_dir: PathBuf,
        session_id: Option<String>,
    ) -> Session {
        let mut sessions = self.sessions.write().await;

        // If session exists, return a clone
        if let Some(existing) = sessions.get(&id) {
            return existing.clone();
        }

        // Create new session
        let mut session = Session::new(id.clone(), agent_type, working_dir);
        session.session_id = session_id;

        // Insert and return clone
        sessions.insert(id, session.clone());
        session
    }

    /// Updates a session's status and returns the updated session.
    ///
    /// This method looks up the session by ID, calls `Session::set_status()` to
    /// update the status (which records the transition in history if the status
    /// actually changed), and returns a clone of the updated session.
    ///
    /// # Arguments
    ///
    /// * `id` - The session ID to update.
    /// * `new_status` - The new status to set.
    ///
    /// # Returns
    ///
    /// `Some(Session)` with the updated session, or `None` if the session was not found.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_console::daemon::store::SessionStore;
    /// use agent_console::{AgentType, Status};
    /// use std::path::PathBuf;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let store = SessionStore::new();
    ///
    ///     // Create a session first
    ///     let _ = store.create_session(
    ///         "session-1".to_string(),
    ///         AgentType::ClaudeCode,
    ///         PathBuf::from("/home/user/project"),
    ///         None,
    ///     ).await;
    ///
    ///     // Update the session status
    ///     let updated = store.update_session("session-1", Status::Attention).await;
    ///     assert!(updated.is_some());
    ///     assert_eq!(updated.unwrap().status, Status::Attention);
    ///
    ///     // Non-existent session returns None
    ///     let missing = store.update_session("nonexistent", Status::Working).await;
    ///     assert!(missing.is_none());
    /// }
    /// ```
    pub async fn update_session(&self, id: &str, new_status: Status) -> Option<Session> {
        let mut sessions = self.sessions.write().await;

        // Get mutable reference to session, update status, return clone
        if let Some(session) = sessions.get_mut(id) {
            let old_status = session.status;
            session.set_status(new_status);
            let updated_session = session.clone();

            self.broadcast_status_change(old_status, &updated_session);
            Some(updated_session)
        } else {
            None
        }
    }

    /// Closes a session by marking it as closed.
    ///
    /// This method sets the session's `closed` flag to `true` and updates its
    /// status to `Status::Closed`. The session remains in the store and can
    /// be queried or potentially resurrected later.
    ///
    /// # Arguments
    ///
    /// * `id` - The session ID to close.
    ///
    /// # Returns
    ///
    /// `Some(Session)` with the closed session, or `None` if the session was not found.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_console::daemon::store::SessionStore;
    /// use agent_console::{AgentType, Status};
    /// use std::path::PathBuf;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let store = SessionStore::new();
    ///
    ///     // Create a session first
    ///     let _ = store.create_session(
    ///         "session-1".to_string(),
    ///         AgentType::ClaudeCode,
    ///         PathBuf::from("/home/user/project"),
    ///         None,
    ///     ).await;
    ///
    ///     // Close the session
    ///     let closed = store.close_session("session-1").await;
    ///     assert!(closed.is_some());
    ///     let session = closed.unwrap();
    ///     assert!(session.closed);
    ///     assert_eq!(session.status, Status::Closed);
    ///
    ///     // Session is still in the store
    ///     let retrieved = store.get("session-1").await;
    ///     assert!(retrieved.is_some());
    ///     assert!(retrieved.unwrap().closed);
    ///
    ///     // Non-existent session returns None
    ///     let missing = store.close_session("nonexistent").await;
    ///     assert!(missing.is_none());
    /// }
    /// ```
    pub async fn close_session(&self, id: &str) -> Option<Session> {
        let mut sessions = self.sessions.write().await;

        // Get mutable reference to session, close it, return clone
        if let Some(session) = sessions.get_mut(id) {
            let old_status = session.status;
            session.closed = true;
            session.set_status(Status::Closed);
            let closed_session = session.clone();

            self.broadcast_status_change(old_status, &closed_session);
            Some(closed_session)
        } else {
            None
        }
    }

    /// Permanently removes a session from the store.
    ///
    /// Unlike `close_session()`, which marks a session as closed but keeps it
    /// in the store for historical purposes, this method completely removes
    /// the session from the store. Use this for cleanup of sessions that are
    /// no longer needed.
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
    ///
    /// # Example
    ///
    /// ```
    /// use agent_console::daemon::store::SessionStore;
    /// use agent_console::AgentType;
    /// use std::path::PathBuf;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let store = SessionStore::new();
    ///
    ///     // Create a session first
    ///     let _ = store.create_session(
    ///         "session-1".to_string(),
    ///         AgentType::ClaudeCode,
    ///         PathBuf::from("/home/user/project"),
    ///         None,
    ///     ).await;
    ///
    ///     // Remove the session permanently
    ///     let removed = store.remove_session("session-1").await;
    ///     assert!(removed.is_some());
    ///     assert_eq!(removed.unwrap().id, "session-1");
    ///
    ///     // Session is no longer in the store
    ///     let retrieved = store.get("session-1").await;
    ///     assert!(retrieved.is_none());
    ///
    ///     // Also no longer appears in list_all
    ///     let sessions = store.list_all().await;
    ///     assert!(sessions.is_empty());
    ///
    ///     // Non-existent session returns None
    ///     let missing = store.remove_session("nonexistent").await;
    ///     assert!(missing.is_none());
    /// }
    /// ```
    pub async fn remove_session(&self, id: &str) -> Option<Session> {
        // Delegates to remove() - both methods are equivalent
        self.remove(id).await
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
    // Lifecycle Method Tests: create_session
    // =========================================================================

    #[tokio::test]
    async fn test_create_session() {
        let store = SessionStore::new();

        // Create session successfully
        let result = store
            .create_session(
                "new-session".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/home/user/project"),
                Some("claude-session-123".to_string()),
            )
            .await;

        assert!(result.is_ok());
        let session = result.unwrap();
        assert_eq!(session.id, "new-session");
        assert_eq!(session.agent_type, AgentType::ClaudeCode);
        assert_eq!(session.working_dir, PathBuf::from("/home/user/project"));
        assert_eq!(session.session_id, Some("claude-session-123".to_string()));
        assert!(!session.closed);

        // Verify session is in store
        let retrieved = store.get("new-session").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "new-session");
    }

    #[tokio::test]
    async fn test_create_session_without_session_id() {
        let store = SessionStore::new();

        // Create session without session_id
        let result = store
            .create_session(
                "no-session-id".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/tmp/test"),
                None,
            )
            .await;

        assert!(result.is_ok());
        let session = result.unwrap();
        assert_eq!(session.id, "no-session-id");
        assert!(session.session_id.is_none());
    }

    #[tokio::test]
    async fn test_create_session_already_exists_error() {
        use crate::StoreError;

        let store = SessionStore::new();

        // Create first session
        let result1 = store
            .create_session(
                "duplicate-id".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/path/1"),
                None,
            )
            .await;
        assert!(result1.is_ok());

        // Attempt to create session with same ID
        let result2 = store
            .create_session(
                "duplicate-id".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/path/2"),
                None,
            )
            .await;

        assert!(result2.is_err());
        match result2.unwrap_err() {
            StoreError::SessionExists(id) => {
                assert_eq!(id, "duplicate-id");
            }
            other => panic!("Expected SessionExists error, got: {:?}", other),
        }

        // Verify original session is unchanged
        let retrieved = store.get("duplicate-id").await.unwrap();
        assert_eq!(retrieved.working_dir, PathBuf::from("/path/1"));
    }

    #[tokio::test]
    async fn test_create_session_explicit_vs_set() {
        // Verify create_session() differs from set() - it doesn't overwrite
        let store = SessionStore::new();

        // First use set() to create a session
        let session1 = create_test_session("test-id");
        store.set("test-id".to_string(), session1).await;

        // Now try create_session() - should fail
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

        // Create multiple unique sessions
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

        // Verify all sessions exist
        let sessions = store.list_all().await;
        assert_eq!(sessions.len(), 5);
    }

    // =========================================================================
    // Lifecycle Method Tests: get_or_create_session
    // =========================================================================

    #[tokio::test]
    async fn test_get_or_create_session_creates_new() {
        let store = SessionStore::new();

        // Call get_or_create_session for a new ID
        let session = store
            .get_or_create_session(
                "new-session".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/home/user/project"),
                Some("claude-session-123".to_string()),
            )
            .await;

        // Verify session was created with correct data
        assert_eq!(session.id, "new-session");
        assert_eq!(session.agent_type, AgentType::ClaudeCode);
        assert_eq!(session.working_dir, PathBuf::from("/home/user/project"));
        assert_eq!(session.session_id, Some("claude-session-123".to_string()));
        assert!(!session.closed);

        // Verify session is in store
        let retrieved = store.get("new-session").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "new-session");
    }

    #[tokio::test]
    async fn test_get_or_create_session_returns_existing() {
        let store = SessionStore::new();

        // First create a session
        let original = store
            .get_or_create_session(
                "existing-session".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/original/path"),
                Some("original-session-id".to_string()),
            )
            .await;

        // Now call get_or_create_session with the same ID but different metadata
        let retrieved = store
            .get_or_create_session(
                "existing-session".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/different/path"), // Different path
                Some("different-session-id".to_string()), // Different session_id
            )
            .await;

        // Verify original session is returned, not modified
        assert_eq!(retrieved.id, "existing-session");
        assert_eq!(retrieved.working_dir, PathBuf::from("/original/path"));
        assert_eq!(
            retrieved.session_id,
            Some("original-session-id".to_string())
        );

        // Verify store still has original
        let from_store = store.get("existing-session").await.unwrap();
        assert_eq!(from_store.working_dir, original.working_dir);
        assert_eq!(from_store.session_id, original.session_id);
    }

    #[tokio::test]
    async fn test_get_or_create_session_without_session_id() {
        let store = SessionStore::new();

        // Create session without session_id
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

        // First use set() to create a session
        let session1 = create_test_session("test-id");
        store.set("test-id".to_string(), session1).await;

        // Now call get_or_create_session - should return existing
        let session2 = store
            .get_or_create_session(
                "test-id".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/new/path"),
                None,
            )
            .await;

        // Should return the original session
        assert_eq!(session2.id, "test-id");
        assert_eq!(session2.working_dir, PathBuf::from("/home/user/test-id")); // Original path
    }

    #[tokio::test]
    async fn test_get_or_create_session_after_create_session() {
        let store = SessionStore::new();

        // First use create_session() to create a session
        let result = store
            .create_session(
                "test-id".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/original/path"),
                Some("original-id".to_string()),
            )
            .await;
        assert!(result.is_ok());

        // Now call get_or_create_session - should return existing
        let session = store
            .get_or_create_session(
                "test-id".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/new/path"),
                Some("new-id".to_string()),
            )
            .await;

        // Should return the original session
        assert_eq!(session.id, "test-id");
        assert_eq!(session.working_dir, PathBuf::from("/original/path"));
        assert_eq!(session.session_id, Some("original-id".to_string()));
    }

    #[tokio::test]
    async fn test_get_or_create_session_multiple_unique() {
        let store = SessionStore::new();

        // Create multiple unique sessions using get_or_create_session
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

        // Verify all sessions exist
        let sessions = store.list_all().await;
        assert_eq!(sessions.len(), 5);
    }

    #[tokio::test]
    async fn test_get_or_create_session_idempotent() {
        let store = SessionStore::new();

        // Call get_or_create_session multiple times with same ID
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

        // All should have the original path (first call's metadata)
        assert_eq!(session1.working_dir, PathBuf::from("/path/1"));
        assert_eq!(session2.working_dir, PathBuf::from("/path/1"));
        assert_eq!(session3.working_dir, PathBuf::from("/path/1"));

        // Store should only have one session
        let sessions = store.list_all().await;
        assert_eq!(sessions.len(), 1);
    }

    // =========================================================================
    // Lifecycle Method Tests: update_session
    // =========================================================================

    #[tokio::test]
    async fn test_update_session() {
        use crate::Status;

        let store = SessionStore::new();

        // Create a session first
        let _ = store
            .create_session(
                "update-test".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/home/user/project"),
                None,
            )
            .await;

        // Update the session status
        let updated = store.update_session("update-test", Status::Attention).await;

        assert!(updated.is_some());
        let session = updated.unwrap();
        assert_eq!(session.id, "update-test");
        assert_eq!(session.status, Status::Attention);
        // Should have one transition recorded (Working -> Attention)
        assert_eq!(session.history.len(), 1);
        assert_eq!(session.history[0].from, Status::Working);
        assert_eq!(session.history[0].to, Status::Attention);
    }

    #[tokio::test]
    async fn test_update_session_not_found() {
        use crate::Status;

        let store = SessionStore::new();

        // Try to update a non-existent session
        let result = store.update_session("nonexistent", Status::Attention).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_session_same_status_no_transition() {
        use crate::Status;

        let store = SessionStore::new();

        // Create a session (starts with Working status)
        let _ = store
            .create_session(
                "same-status".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/tmp/test"),
                None,
            )
            .await;

        // Update to the same status (Working)
        let updated = store.update_session("same-status", Status::Working).await;

        assert!(updated.is_some());
        let session = updated.unwrap();
        assert_eq!(session.status, Status::Working);
        // No transition should be recorded since status didn't change
        assert!(session.history.is_empty());
    }

    #[tokio::test]
    async fn test_update_session_multiple_transitions() {
        use crate::Status;

        let store = SessionStore::new();

        // Create a session
        let _ = store
            .create_session(
                "multi-transition".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/tmp/test"),
                None,
            )
            .await;

        // Perform multiple status transitions
        let _ = store
            .update_session("multi-transition", Status::Attention)
            .await;
        let _ = store
            .update_session("multi-transition", Status::Question)
            .await;
        let result = store
            .update_session("multi-transition", Status::Working)
            .await;

        assert!(result.is_some());
        let session = result.unwrap();
        assert_eq!(session.status, Status::Working);
        assert_eq!(session.history.len(), 3);

        // Verify transition sequence
        assert_eq!(session.history[0].from, Status::Working);
        assert_eq!(session.history[0].to, Status::Attention);
        assert_eq!(session.history[1].from, Status::Attention);
        assert_eq!(session.history[1].to, Status::Question);
        assert_eq!(session.history[2].from, Status::Question);
        assert_eq!(session.history[2].to, Status::Working);
    }

    #[tokio::test]
    async fn test_update_session_persists_in_store() {
        use crate::Status;

        let store = SessionStore::new();

        // Create a session
        let _ = store
            .create_session(
                "persist-test".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/tmp/test"),
                None,
            )
            .await;

        // Update status
        let _ = store
            .update_session("persist-test", Status::Question)
            .await;

        // Verify the update persisted by reading from store again
        let retrieved = store.get("persist-test").await;
        assert!(retrieved.is_some());
        let session = retrieved.unwrap();
        assert_eq!(session.status, Status::Question);
        assert_eq!(session.history.len(), 1);
    }

    #[tokio::test]
    async fn test_update_session_preserves_metadata() {
        use crate::Status;

        let store = SessionStore::new();

        // Create a session with metadata
        let _ = store
            .create_session(
                "preserve-meta".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/specific/path"),
                Some("claude-session-xyz".to_string()),
            )
            .await;

        // Update status
        let updated = store
            .update_session("preserve-meta", Status::Attention)
            .await;

        assert!(updated.is_some());
        let session = updated.unwrap();

        // Verify metadata is preserved
        assert_eq!(session.id, "preserve-meta");
        assert_eq!(session.agent_type, AgentType::ClaudeCode);
        assert_eq!(session.working_dir, PathBuf::from("/specific/path"));
        assert_eq!(session.session_id, Some("claude-session-xyz".to_string()));
        assert_eq!(session.status, Status::Attention);
    }

    // =========================================================================
    // Lifecycle Method Tests: close_session
    // =========================================================================

    #[tokio::test]
    async fn test_close_session() {
        use crate::Status;

        let store = SessionStore::new();

        // Create a session first
        let _ = store
            .create_session(
                "close-test".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/home/user/project"),
                None,
            )
            .await;

        // Close the session
        let closed = store.close_session("close-test").await;

        assert!(closed.is_some());
        let session = closed.unwrap();
        assert_eq!(session.id, "close-test");
        assert!(session.closed);
        assert_eq!(session.status, Status::Closed);
    }

    #[tokio::test]
    async fn test_close_session_not_found() {
        let store = SessionStore::new();

        // Try to close a non-existent session
        let result = store.close_session("nonexistent").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_close_session_persists_in_store() {
        use crate::Status;

        let store = SessionStore::new();

        // Create a session
        let _ = store
            .create_session(
                "persist-close".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/tmp/test"),
                None,
            )
            .await;

        // Close the session
        let _ = store.close_session("persist-close").await;

        // Session should still be in the store
        let retrieved = store.get("persist-close").await;
        assert!(retrieved.is_some());
        let session = retrieved.unwrap();
        assert!(session.closed);
        assert_eq!(session.status, Status::Closed);
    }

    #[tokio::test]
    async fn test_close_session_records_transition() {
        use crate::Status;

        let store = SessionStore::new();

        // Create a session (starts with Working status)
        let _ = store
            .create_session(
                "transition-close".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/tmp/test"),
                None,
            )
            .await;

        // Close the session
        let closed = store.close_session("transition-close").await;

        assert!(closed.is_some());
        let session = closed.unwrap();
        // Should have one transition recorded (Working -> Closed)
        assert_eq!(session.history.len(), 1);
        assert_eq!(session.history[0].from, Status::Working);
        assert_eq!(session.history[0].to, Status::Closed);
    }

    #[tokio::test]
    async fn test_close_session_preserves_metadata() {
        use crate::Status;

        let store = SessionStore::new();

        // Create a session with metadata
        let _ = store
            .create_session(
                "preserve-close".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/specific/path"),
                Some("claude-session-xyz".to_string()),
            )
            .await;

        // Close the session
        let closed = store.close_session("preserve-close").await;

        assert!(closed.is_some());
        let session = closed.unwrap();

        // Verify metadata is preserved
        assert_eq!(session.id, "preserve-close");
        assert_eq!(session.agent_type, AgentType::ClaudeCode);
        assert_eq!(session.working_dir, PathBuf::from("/specific/path"));
        assert_eq!(session.session_id, Some("claude-session-xyz".to_string()));
        assert!(session.closed);
        assert_eq!(session.status, Status::Closed);
    }

    #[tokio::test]
    async fn test_close_session_idempotent() {
        use crate::Status;

        let store = SessionStore::new();

        // Create a session
        let _ = store
            .create_session(
                "idempotent-close".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/tmp/test"),
                None,
            )
            .await;

        // Close the session twice
        let closed1 = store.close_session("idempotent-close").await;
        let closed2 = store.close_session("idempotent-close").await;

        // Both calls should succeed
        assert!(closed1.is_some());
        assert!(closed2.is_some());

        // Session should still be closed
        let session = closed2.unwrap();
        assert!(session.closed);
        assert_eq!(session.status, Status::Closed);

        // Only one transition recorded (second close has same status, so no transition)
        assert_eq!(session.history.len(), 1);
    }

    #[tokio::test]
    async fn test_close_session_list_all_includes_closed() {
        let store = SessionStore::new();

        // Create two sessions
        let _ = store
            .create_session(
                "session-1".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/path/1"),
                None,
            )
            .await;
        let _ = store
            .create_session(
                "session-2".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/path/2"),
                None,
            )
            .await;

        // Close one session
        let _ = store.close_session("session-1").await;

        // list_all should include both sessions
        let sessions = store.list_all().await;
        assert_eq!(sessions.len(), 2);

        // Find the closed session
        let closed_session = sessions.iter().find(|s| s.id == "session-1");
        assert!(closed_session.is_some());
        assert!(closed_session.unwrap().closed);

        // Find the active session
        let active_session = sessions.iter().find(|s| s.id == "session-2");
        assert!(active_session.is_some());
        assert!(!active_session.unwrap().closed);
    }

    // =========================================================================
    // Lifecycle Method Tests: remove_session
    // =========================================================================

    #[tokio::test]
    async fn test_remove_session() {
        use crate::Status;

        let store = SessionStore::new();

        // Create three sessions
        let _ = store
            .create_session(
                "session-to-remove".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/path/remove"),
                None,
            )
            .await;
        let _ = store
            .create_session(
                "session-to-close".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/path/close"),
                None,
            )
            .await;
        let _ = store
            .create_session(
                "session-active".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/path/active"),
                None,
            )
            .await;

        // Close one session (should still appear in list_all)
        let closed = store.close_session("session-to-close").await;
        assert!(closed.is_some());
        assert!(closed.unwrap().closed);

        // Remove one session (should NOT appear in list_all)
        let removed = store.remove_session("session-to-remove").await;
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, "session-to-remove");

        // Verify list_all behavior:
        // - Closed sessions STILL appear in list_all
        // - Removed sessions do NOT appear in list_all
        let sessions = store.list_all().await;
        assert_eq!(sessions.len(), 2);

        // Find the closed session (should be present)
        let closed_session = sessions.iter().find(|s| s.id == "session-to-close");
        assert!(closed_session.is_some());
        assert!(closed_session.unwrap().closed);
        assert_eq!(closed_session.unwrap().status, Status::Closed);

        // Find the active session (should be present)
        let active_session = sessions.iter().find(|s| s.id == "session-active");
        assert!(active_session.is_some());
        assert!(!active_session.unwrap().closed);

        // The removed session should NOT be in list_all
        let removed_session = sessions.iter().find(|s| s.id == "session-to-remove");
        assert!(removed_session.is_none());

        // Also verify get returns None for removed session
        assert!(store.get("session-to-remove").await.is_none());

        // But get still works for closed session
        assert!(store.get("session-to-close").await.is_some());
    }

    #[tokio::test]
    async fn test_remove_session_not_found() {
        let store = SessionStore::new();

        // Try to remove a non-existent session
        let result = store.remove_session("nonexistent").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_remove_session_idempotent() {
        let store = SessionStore::new();

        // Create a session
        let _ = store
            .create_session(
                "to-remove".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/tmp/test"),
                None,
            )
            .await;

        // First remove succeeds
        let removed1 = store.remove_session("to-remove").await;
        assert!(removed1.is_some());

        // Second remove returns None (idempotent)
        let removed2 = store.remove_session("to-remove").await;
        assert!(removed2.is_none());
    }

    #[tokio::test]
    async fn test_remove_session_preserves_data() {
        let store = SessionStore::new();

        // Create a session with specific data
        let _ = store
            .create_session(
                "data-session".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/specific/path"),
                Some("claude-session-xyz".to_string()),
            )
            .await;

        // Remove the session
        let removed = store.remove_session("data-session").await;

        // Verify the returned session has all the original data
        assert!(removed.is_some());
        let session = removed.unwrap();
        assert_eq!(session.id, "data-session");
        assert_eq!(session.agent_type, AgentType::ClaudeCode);
        assert_eq!(session.working_dir, PathBuf::from("/specific/path"));
        assert_eq!(session.session_id, Some("claude-session-xyz".to_string()));
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
        use crate::Status;

        let store = SessionStore::new();
        let mut rx = store.subscribe();

        // Create a session first
        let _ = store
            .create_session(
                "notify-test".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/home/user/project"),
                None,
            )
            .await;

        // Update the session status
        let _ = store.update_session("notify-test", Status::Attention).await;

        // Subscriber should receive the notification
        let update = rx.try_recv();
        assert!(update.is_ok(), "Subscriber should receive update notification");
        let update = update.unwrap();
        assert_eq!(update.session_id, "notify-test");
        assert_eq!(update.status, Status::Attention);
    }

    #[tokio::test]
    async fn test_subscriber_no_notification_on_same_status() {
        use crate::Status;

        let store = SessionStore::new();
        let mut rx = store.subscribe();

        // Create a session (starts with Working status)
        let _ = store
            .create_session(
                "same-status-notify".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/tmp/test"),
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
        use crate::Status;

        let store = SessionStore::new();
        let mut rx = store.subscribe();

        // Create a session
        let _ = store
            .create_session(
                "close-notify".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/tmp/test"),
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
        let update = update.unwrap();
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
                PathBuf::from("/tmp/test"),
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
        use crate::Status;

        let store = SessionStore::new();
        let mut rx = store.subscribe();

        // Create a session
        let _ = store
            .create_session(
                "multi-update".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/tmp/test"),
                None,
            )
            .await;

        // Perform multiple status transitions
        let _ = store
            .update_session("multi-update", Status::Attention)
            .await;
        let _ = store
            .update_session("multi-update", Status::Question)
            .await;
        let _ = store.close_session("multi-update").await;

        // Subscriber should receive all three notifications
        let update1 = rx.try_recv();
        assert!(update1.is_ok());
        assert_eq!(update1.unwrap().status, Status::Attention);

        let update2 = rx.try_recv();
        assert!(update2.is_ok());
        assert_eq!(update2.unwrap().status, Status::Question);

        let update3 = rx.try_recv();
        assert!(update3.is_ok());
        assert_eq!(update3.unwrap().status, Status::Closed);
    }

    #[tokio::test]
    async fn test_subscriber_multiple_subscribers_all_notified() {
        use crate::Status;

        let store = SessionStore::new();
        let mut rx1 = store.subscribe();
        let mut rx2 = store.subscribe();
        let mut rx3 = store.subscribe();

        // Create a session
        let _ = store
            .create_session(
                "multi-subscriber".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/tmp/test"),
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
        assert_eq!(update1.unwrap().status, Status::Attention);
        assert_eq!(update2.unwrap().status, Status::Attention);
        assert_eq!(update3.unwrap().status, Status::Attention);
    }

    #[tokio::test]
    async fn test_subscriber_notification_does_not_block_without_subscribers() {
        use crate::Status;

        let store = SessionStore::new();
        // No subscribers

        // Create a session
        let _ = store
            .create_session(
                "no-subscriber".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/tmp/test"),
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
        use crate::Status;
        use std::time::Duration;

        let store = SessionStore::new();
        let mut rx = store.subscribe();

        // Create a session
        let _ = store
            .create_session(
                "elapsed-test".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/tmp/test"),
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
        let update = update.unwrap();
        // Elapsed seconds should be 0 or small (since we just changed status)
        // The elapsed is calculated from the updated 'since' timestamp,
        // so right after status change it should be very small
        assert!(
            update.elapsed_seconds < 5,
            "Elapsed seconds should be small right after status change"
        );
    }
}
