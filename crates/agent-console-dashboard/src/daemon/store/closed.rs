//! Closed session operations for the SessionStore.
//!
//! This module contains methods for managing closed sessions and session removal.

use super::SessionStore;
use crate::daemon::session::ClosedSession;
use crate::{Session, Status};
use std::time::Duration;

impl SessionStore {
    /// Closes a session by marking it as closed.
    ///
    /// This method sets the session's `closed` flag to `true` and updates its
    /// status to `Status::Closed`. The session remains in the store and can
    /// be queried or potentially reopened later.
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
    /// use agent_console_dashboard::daemon::store::SessionStore;
    /// use agent_console_dashboard::{AgentType, Status};
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
    ///         Some(PathBuf::from("/home/user/project")),
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
                let old_priority = session.priority;
                session.closed = true;
                session.set_status(Status::Closed);
                let result = session.clone();
                self.broadcast_session_change(old_status, old_priority, &result);
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

    /// Returns the count of non-closed sessions that have been inactive
    /// (no hook activity) for longer than `threshold`.
    pub async fn count_inactive_sessions(&self, threshold: Duration) -> usize {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|s| s.is_inactive(threshold))
            .count()
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
    /// Used during reopen to remove the session from the closed queue.
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
    /// use agent_console_dashboard::daemon::store::SessionStore;
    /// use agent_console_dashboard::AgentType;
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
    ///         Some(PathBuf::from("/home/user/project")),
    ///         None,
    ///     ).await;
    ///
    ///     // Remove the session permanently
    ///     let removed = store.remove_session("session-1").await;
    ///     assert!(removed.is_some());
    ///     assert_eq!(removed.unwrap().session_id, "session-1");
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
