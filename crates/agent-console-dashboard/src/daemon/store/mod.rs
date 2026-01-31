//! Session store module for the Agent Console daemon.
//!
//! This module provides a thread-safe, in-memory session store for tracking
//! all active agent sessions. It uses `Arc<RwLock<HashMap>>` for O(1) lookups
//! by session ID while supporting concurrent access from multiple async tasks.

use crate::daemon::session::ClosedSession;
use crate::{AgentType, Session, SessionUpdate, Status, StoreError};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, RwLock};

#[cfg(test)]
mod tests;

/// Default capacity for the subscriber notification channel.
/// This allows for bursty update scenarios without dropping notifications.
const DEFAULT_SUBSCRIBER_CHANNEL_CAPACITY: usize = 256;

/// Default maximum count of closed sessions to retain.
const DEFAULT_MAX_CLOSED_SESSIONS: usize = 20;

/// Thread-safe session store wrapping a HashMap with `Arc<RwLock>`.
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
    /// Internal session storage wrapped in `Arc<RwLock>` for thread-safe access.
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    /// Broadcast channel sender for subscriber notifications.
    /// Subscribers receive [`SessionUpdate`] messages on state changes.
    update_tx: broadcast::Sender<SessionUpdate>,
    /// Closed session metadata for resurrection, ordered by close time.
    closed: Arc<RwLock<VecDeque<ClosedSession>>>,
    /// Maximum count of closed sessions to retain before evicting oldest.
    max_closed_sessions: usize,
    /// Daemon start time for computing elapsed seconds in closed sessions.
    daemon_start: Instant,
}

impl std::fmt::Debug for SessionStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionStore")
            .field("sessions", &self.sessions)
            .field("subscriber_count", &self.update_tx.receiver_count())
            .field("closed", &self.closed)
            .field("max_closed_sessions", &self.max_closed_sessions)
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
            closed: Arc::new(RwLock::new(VecDeque::new())),
            max_closed_sessions: DEFAULT_MAX_CLOSED_SESSIONS,
            daemon_start: Instant::now(),
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
        let closed_session = {
            let mut sessions = self.sessions.write().await;

            if let Some(session) = sessions.get_mut(id) {
                let old_status = session.status;
                session.closed = true;
                session.set_status(Status::Closed);
                let result = session.clone();
                self.broadcast_status_change(old_status, &result);
                Some(result)
            } else {
                None
            }
        };

        // Store closed session metadata outside the sessions lock
        if let Some(ref session) = closed_session {
            let closed_meta = ClosedSession::from_session(session, self.daemon_start);
            let mut closed_queue = self.closed.write().await;

            // Deduplicate: remove existing entry for same session ID
            closed_queue.retain(|c| c.session_id != id);
            closed_queue.push_back(closed_meta);

            // Enforce retention limit
            while closed_queue.len() > self.max_closed_sessions {
                closed_queue.pop_front();
            }
        }

        closed_session
    }

    /// Returns closed sessions sorted by close time (most recent first).
    pub async fn list_closed(&self) -> Vec<ClosedSession> {
        let closed = self.closed.read().await;
        let mut result: Vec<ClosedSession> = closed.iter().cloned().collect();
        result.reverse(); // VecDeque has oldest first, we want most recent first
        result
    }

    /// Retrieves a closed session by its session ID.
    ///
    /// Returns `None` if no closed session with the given ID exists.
    pub async fn get_closed(&self, session_id: &str) -> Option<ClosedSession> {
        let closed = self.closed.read().await;
        closed.iter().find(|c| c.session_id == session_id).cloned()
    }

    /// Removes a closed session by its session ID.
    ///
    /// Used during resurrection to remove the session from the closed queue.
    /// Returns `Some(ClosedSession)` if found and removed, `None` otherwise.
    pub async fn remove_closed(&self, session_id: &str) -> Option<ClosedSession> {
        let mut closed = self.closed.write().await;
        if let Some(pos) = closed.iter().position(|c| c.session_id == session_id) {
            closed.remove(pos)
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
