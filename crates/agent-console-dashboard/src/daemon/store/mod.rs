//! Session store module for the Agent Console daemon.
//!
//! This module provides a thread-safe, in-memory session store for tracking
//! all active agent sessions. It uses `Arc<RwLock<HashMap>>` for O(1) lookups
//! by session ID while supporting concurrent access from multiple async tasks.

use crate::daemon::session::ClosedSession;
use crate::{Session, SessionUpdate, Status};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};

#[cfg(test)]
mod tests;

mod closed;
mod lifecycle;

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
/// use agent_console_dashboard::daemon::store::SessionStore;
/// use agent_console_dashboard::{Session, AgentType};
/// use std::path::PathBuf;
///
/// #[tokio::main]
/// async fn main() {
///     let store = SessionStore::new();
///     let session = Session::new(
///         "session-1".to_string(),
///         AgentType::ClaudeCode,
///         Some(PathBuf::from("/home/user/project")),
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
    /// Closed session metadata for reopen, ordered by close time.
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
    /// use agent_console_dashboard::daemon::store::SessionStore;
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

    /// Broadcasts a session change notification to all subscribers.
    ///
    /// Sends if status or priority changed. Logs the result at trace/debug level.
    pub(super) fn broadcast_session_change(
        &self,
        old_status: Status,
        old_priority: u64,
        session: &Session,
    ) {
        if old_status != session.status || old_priority != session.priority {
            let update = SessionUpdate::new(
                session.session_id.clone(),
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
    /// use agent_console_dashboard::daemon::store::SessionStore;
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

    /// Returns `true` if any session is both non-closed and not inactive.
    ///
    /// Sessions that have received no hook activity for longer than
    /// `inactive_threshold` are considered inactive and excluded.
    pub async fn has_active_sessions(&self, inactive_threshold: Duration) -> bool {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .any(|s| s.status != Status::Closed && !s.is_inactive(inactive_threshold))
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
