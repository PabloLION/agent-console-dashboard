//! Session lifecycle operations for the SessionStore.
//!
//! This module contains methods for creating and updating sessions during their
//! active lifecycle.

use super::SessionStore;
use crate::{AgentType, Session, Status, StoreError};
use std::path::PathBuf;

impl SessionStore {
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
    /// use agent_console_dashboard::daemon::store::SessionStore;
    /// use agent_console_dashboard::AgentType;
    /// use std::path::PathBuf;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let store = SessionStore::new();
    ///     let result = store.create_session(
    ///         "session-1".to_string(),
    ///         AgentType::ClaudeCode,
    ///         Some(PathBuf::from("/home/user/project")),
    ///         Some("claude-session-abc".to_string()),
    ///     ).await;
    ///     assert!(result.is_ok());
    ///
    ///     // Attempting to create again returns error
    ///     let result2 = store.create_session(
    ///         "session-1".to_string(),
    ///         AgentType::ClaudeCode,
    ///         Some(PathBuf::from("/home/user/project")),
    ///         None,
    ///     ).await;
    ///     assert!(result2.is_err());
    /// }
    /// ```
    pub async fn create_session(
        &self,
        id: String,
        agent_type: AgentType,
        working_dir: Option<PathBuf>,
        _session_id: Option<String>,
    ) -> Result<Session, StoreError> {
        let mut sessions = self.sessions.write().await;

        // Check if session already exists
        if sessions.contains_key(&id) {
            return Err(StoreError::SessionExists(id));
        }

        // Create new session
        let session = Session::new(id.clone(), agent_type, working_dir);

        // Insert and return clone
        sessions.insert(id, session.clone());
        Ok(session)
    }

    /// Gets existing session or creates new one if not found, and sets status.
    ///
    /// This method is used for lazy session creation on first SET command.
    /// Unlike `create_session()`, this method never fails - it always returns
    /// a valid session. If a session with the given ID exists, its status is
    /// updated. Otherwise, a new session is created with the provided metadata
    /// and status. Both operations happen under a single lock acquisition,
    /// eliminating TOCTOU races between create and update.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique session identifier.
    /// * `agent_type` - Type of agent (e.g., ClaudeCode).
    /// * `working_dir` - Working directory path for this session.
    /// * `session_id` - Optional Claude Code session ID for resume capability.
    /// * `status` - Status to set on the session.
    ///
    /// # Returns
    ///
    /// The existing (updated) or newly created session.
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
    ///     // First call creates a new session with status
    ///     let session1 = store.get_or_create_session(
    ///         "session-1".to_string(),
    ///         AgentType::ClaudeCode,
    ///         Some(PathBuf::from("/home/user/project")),
    ///         Some("claude-session-abc".to_string()),
    ///         Status::Working,
    ///     ).await;
    ///     assert_eq!(session1.session_id, "session-1");
    ///     assert_eq!(session1.status, Status::Working);
    ///
    ///     // Second call updates status on existing session
    ///     let session2 = store.get_or_create_session(
    ///         "session-1".to_string(),
    ///         AgentType::ClaudeCode,
    ///         Some(PathBuf::from("/different/path")),
    ///         None,
    ///         Status::Attention,
    ///     ).await;
    ///     assert_eq!(session2.status, Status::Attention);
    /// }
    /// ```
    pub async fn get_or_create_session(
        &self,
        id: String,
        agent_type: AgentType,
        working_dir: Option<PathBuf>,
        _session_id: Option<String>,
        status: Status,
    ) -> Session {
        let mut sessions = self.sessions.write().await;

        // If session exists, update status and working_dir, then return
        if let Some(existing) = sessions.get_mut(&id) {
            let old_status = existing.status;
            // Update working_dir if the caller provides Some(path)
            if working_dir.is_some() {
                existing.working_dir = working_dir;
            }
            existing.set_status(status);
            let updated = existing.clone();
            self.broadcast_status_change(old_status, &updated);
            return updated;
        }

        // Create new session with the requested status
        let mut session = Session::new(id.clone(), agent_type, working_dir);
        session.set_status(status);

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
}
