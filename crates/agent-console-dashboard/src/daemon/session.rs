//! Closed session metadata for resurrection support.
//!
//! When a session is closed, its metadata is preserved in a [`ClosedSession`]
//! struct that can be used to resurrect the session later. Since `Instant`
//! cannot be serialized, elapsed seconds from daemon start are stored for
//! serialization, while a runtime-only `Instant` field is kept for sorting.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

use crate::{Session, Status};

/// Metadata for a closed session available for resurrection.
///
/// This struct captures the essential information needed to identify and
/// potentially resume a previously active session. It is stored in the
/// session store's closed session queue with a configurable retention limit.
///
/// # Serialization
///
/// The `closed_at` field is skipped during serialization because `Instant`
/// does not support serde. Instead, `started_at_elapsed` and `closed_at_elapsed`
/// store seconds since daemon start for persistent storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosedSession {
    /// Unique session identifier (matches the original session ID).
    pub session_id: String,
    /// Working directory the session was using.
    pub working_dir: PathBuf,
    /// Seconds since daemon start when the session was created.
    pub started_at_elapsed: u64,
    /// Seconds since daemon start when the session was closed.
    pub closed_at_elapsed: u64,
    /// Whether the session can be resumed.
    pub resumable: bool,
    /// Reason the session cannot be resumed, if applicable.
    pub not_resumable_reason: Option<String>,
    /// Last known status before closing (for display purposes).
    pub last_status: Status,
    /// Runtime-only timestamp for sorting by recency.
    #[serde(skip)]
    pub closed_at: Option<Instant>,
}

impl ClosedSession {
    /// Creates a new `ClosedSession` from an active session being closed.
    ///
    /// The `daemon_start` instant is used to compute elapsed seconds for
    /// serialization. The `closed_at` field is set to `Instant::now()`.
    ///
    /// # Arguments
    ///
    /// * `session` - The session being closed.
    /// * `daemon_start` - The instant when the daemon started (for elapsed computation).
    pub fn from_session(session: &Session, daemon_start: Instant) -> Self {
        let now = Instant::now();
        let started_at_elapsed = session.since.duration_since(daemon_start).as_secs();
        let closed_at_elapsed = now.duration_since(daemon_start).as_secs();

        // Sessions are not resumable (Claude Code session ID tracking removed)
        let (resumable, not_resumable_reason) = (
            false,
            Some("no Claude Code session ID available".to_string()),
        );

        Self {
            session_id: session.session_id.clone(),
            working_dir: session.working_dir.clone(),
            started_at_elapsed,
            closed_at_elapsed,
            resumable,
            not_resumable_reason,
            last_status: session.status,
            closed_at: Some(now),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AgentType;

    fn make_session(id: &str) -> Session {
        Session::new(
            id.to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/tmp/test"),
        )
    }

    #[test]
    fn from_session_captures_all_fields() {
        let daemon_start = Instant::now();
        let session = make_session("s1");
        let closed = ClosedSession::from_session(&session, daemon_start);

        assert_eq!(closed.session_id, "s1");
        assert_eq!(closed.working_dir, PathBuf::from("/tmp/test"));
        assert!(!closed.resumable);
        assert!(closed.not_resumable_reason.is_some());
        assert_eq!(closed.last_status, Status::Working);
        assert!(closed.closed_at.is_some());
    }

    #[test]
    fn from_session_without_session_id_is_not_resumable() {
        let daemon_start = Instant::now();
        let session = make_session("s2");
        let closed = ClosedSession::from_session(&session, daemon_start);

        assert!(!closed.resumable);
        assert!(closed.not_resumable_reason.is_some());
        assert!(closed
            .not_resumable_reason
            .as_ref()
            .expect("reason should exist")
            .contains("no Claude Code session ID"));
    }

    #[test]
    fn elapsed_seconds_are_computed() {
        let daemon_start = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(50));
        let session = make_session("s3");
        let closed = ClosedSession::from_session(&session, daemon_start);

        // closed_at_elapsed should be >= started_at_elapsed
        assert!(closed.closed_at_elapsed >= closed.started_at_elapsed);
    }

    #[test]
    fn serialization_roundtrip() {
        let daemon_start = Instant::now();
        let session = make_session("s4");
        let closed = ClosedSession::from_session(&session, daemon_start);

        let json = serde_json::to_string(&closed).expect("should serialize");
        let deserialized: ClosedSession = serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(deserialized.session_id, "s4");
        assert!(!deserialized.resumable);
        assert_eq!(deserialized.working_dir, PathBuf::from("/tmp/test"));
        // closed_at should be None after deserialization (skipped)
        assert!(deserialized.closed_at.is_none());
    }

    #[test]
    fn clone_preserves_all_fields() {
        let daemon_start = Instant::now();
        let session = make_session("s5");
        let closed = ClosedSession::from_session(&session, daemon_start);
        let cloned = closed.clone();

        assert_eq!(cloned.session_id, closed.session_id);
        assert_eq!(cloned.working_dir, closed.working_dir);
        assert_eq!(cloned.resumable, closed.resumable);
        assert_eq!(cloned.last_status, closed.last_status);
    }

    #[test]
    fn debug_format_contains_session_id() {
        let daemon_start = Instant::now();
        let session = make_session("debug-s");
        let closed = ClosedSession::from_session(&session, daemon_start);
        let debug = format!("{:?}", closed);
        assert!(debug.contains("debug-s"));
    }
}
