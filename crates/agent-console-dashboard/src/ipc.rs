//! IPC wire types for JSON Lines protocol over Unix domain sockets.

use crate::{AgentType, Session, Status};
use std::path::PathBuf;
use std::time::Instant;

/// IPC protocol version. Included in every message for forward/backward
/// compatibility.
pub const IPC_VERSION: u32 = 1;

/// Incoming command from a client to the daemon.
///
/// Every message is a single JSON line:
/// `{"version": 1, "cmd": "SET", ...}\n`
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IpcCommand {
    /// Protocol version (must be [`IPC_VERSION`]).
    pub version: u32,
    /// Command name (SET, LIST, GET, RM, SUB, STATUS, DUMP, RESURRECT, STOP).
    pub cmd: String,
    /// Session identifier (for SET, GET, RM, RESURRECT).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Session status string (for SET).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Working directory (for SET). None if unknown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    /// Confirmation flag (for STOP).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed: Option<bool>,
}

/// Response envelope from daemon to client.
///
/// Sent as a single JSON line: `{"version": 1, "ok": true, ...}\n`
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IpcResponse {
    /// Protocol version.
    pub version: u32,
    /// Whether the command succeeded.
    pub ok: bool,
    /// Error message when `ok` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Command-specific payload (varies by command).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl IpcResponse {
    /// Creates a success response with optional data payload.
    pub fn success(data: Option<serde_json::Value>) -> Self {
        Self {
            version: IPC_VERSION,
            ok: true,
            error: None,
            data,
        }
    }

    /// Creates an error response with the given message.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            version: IPC_VERSION,
            ok: false,
            error: Some(message.into()),
            data: None,
        }
    }

    /// Serializes to a JSON line (with trailing newline).
    pub fn to_json_line(&self) -> String {
        let json = serde_json::to_string(self).expect("failed to serialize IpcResponse");
        format!("{}\n", json)
    }
}

/// Serializable point-in-time view of a session for the IPC wire format.
///
/// Converts from `&Session` (which contains non-serializable `Instant` fields)
/// into a fully serializable struct with elapsed/idle seconds computed at
/// conversion time.
///
/// # Use Cases
///
/// This struct serves three primary purposes:
///
/// 1. **IPC wire format**: Daemon sends session snapshots to TUI consumers via
///    JSON Lines over Unix domain socket (LIST, GET, SUB commands).
/// 2. **Double-click hook payload**: TUI passes session context to user-defined
///    hooks as JSON on stdin when a session is double-clicked.
/// 3. **Public API for hook authors**: Re-exported from the library crate so
///    Rust hook authors can deserialize the JSON payload with `serde_json`.
///
/// See `docs/decisions/variable-naming.md` for naming rationale.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SessionSnapshot {
    /// Session identifier (was `Session.id`).
    pub session_id: String,
    /// Agent type as string (e.g., "claude-code").
    pub agent_type: String,
    /// Current status as lowercase string.
    pub status: String,
    /// Working directory, or None if unknown.
    pub working_dir: Option<String>,
    /// Seconds since the session entered its current status.
    pub elapsed_seconds: u64,
    /// Seconds since last hook activity.
    pub idle_seconds: u64,
    /// State transition history (bounded queue, ~10 entries).
    pub history: Vec<StatusChange>,
    /// Whether session has been closed.
    pub closed: bool,
}

/// A single status change in the history, serializable for IPC.
///
/// Each entry records "became status X at time T". Consumers derive duration
/// (diff between consecutive `at_secs`) and previous status (prior entry's
/// `status`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct StatusChange {
    /// The new status after this transition.
    pub status: String,
    /// Unix timestamp (seconds since epoch) when this status began.
    pub at_secs: u64,
}

impl From<&Session> for SessionSnapshot {
    fn from(session: &Session) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        let working_dir = session
            .working_dir
            .as_ref()
            .map(|p| p.display().to_string());

        let now_instant = Instant::now();
        let now_system = SystemTime::now();
        let history = session
            .history
            .iter()
            .map(|t| {
                // Approximate unix timestamp from monotonic Instant
                let elapsed = now_instant.duration_since(t.timestamp);
                let transition_time = now_system - elapsed;
                let at_secs = transition_time
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                StatusChange {
                    status: t.to.to_string(),
                    at_secs,
                }
            })
            .collect();

        Self {
            session_id: session.session_id.clone(),
            agent_type: format!("{:?}", session.agent_type).to_lowercase(),
            status: session.status.to_string(),
            working_dir,
            elapsed_seconds: session.since.elapsed().as_secs(),
            idle_seconds: session.last_activity.elapsed().as_secs(),
            history,
            closed: session.closed,
        }
    }
}

/// A SUB notification pushed from daemon to subscriber.
///
/// Sent as a single JSON line on the SUB stream.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IpcNotification {
    /// Protocol version.
    pub version: u32,
    /// Notification type: "update", "usage", "warn".
    #[serde(rename = "type")]
    pub notification_type: String,
    /// Full session snapshot (for "update" notifications).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionSnapshot>,
    /// Usage data (for "usage" notifications).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<serde_json::Value>,
    /// Warning message (for "warn" notifications).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl IpcNotification {
    /// Creates an "update" notification with full session snapshot.
    pub fn session_update(info: SessionSnapshot) -> Self {
        Self {
            version: IPC_VERSION,
            notification_type: "update".to_string(),
            session: Some(info),
            usage: None,
            message: None,
        }
    }

    /// Creates a "usage" notification with API usage data.
    pub fn usage_update(data: &claude_usage::UsageData) -> Self {
        Self {
            version: IPC_VERSION,
            notification_type: "usage".to_string(),
            session: None,
            usage: Some(serde_json::to_value(data).expect("failed to serialize UsageData")),
            message: None,
        }
    }

    /// Creates a "warn" notification.
    pub fn warn(message: impl Into<String>) -> Self {
        Self {
            version: IPC_VERSION,
            notification_type: "warn".to_string(),
            session: None,
            usage: None,
            message: Some(message.into()),
        }
    }

    /// Serializes to a JSON line (with trailing newline).
    pub fn to_json_line(&self) -> String {
        let json = serde_json::to_string(self).expect("failed to serialize IpcNotification");
        format!("{}\n", json)
    }
}

/// Metadata parsed from SET command JSON payload.
///
/// This struct is used to pass optional session metadata when creating
/// or updating sessions via the SET command. All fields are optional
/// to allow for partial updates or default values.
#[derive(Debug, Clone, Default)]
pub struct SessionMetadata {
    /// Working directory for this session.
    pub working_dir: Option<PathBuf>,
    /// Claude Code session ID for resume capability.
    pub session_id: Option<String>,
    /// Agent type (defaults to ClaudeCode if not specified).
    pub agent_type: Option<AgentType>,
}

impl SessionMetadata {
    /// Creates a new SessionMetadata with all fields set to None.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a SessionMetadata with the specified working directory.
    pub fn with_working_dir(working_dir: PathBuf) -> Self {
        Self {
            working_dir: Some(working_dir),
            ..Default::default()
        }
    }

    /// Sets the working directory and returns self for chaining.
    pub fn working_dir(mut self, path: PathBuf) -> Self {
        self.working_dir = Some(path);
        self
    }

    /// Sets the session ID and returns self for chaining.
    pub fn session_id(mut self, id: String) -> Self {
        self.session_id = Some(id);
        self
    }

    /// Sets the agent type and returns self for chaining.
    pub fn agent_type(mut self, agent_type: AgentType) -> Self {
        self.agent_type = Some(agent_type);
        self
    }
}

/// Notification payload for session updates sent to subscribers.
///
/// This struct contains the essential information about a session update
/// that gets broadcast to all registered subscribers when a session's
/// state changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionUpdate {
    /// Unique session identifier.
    pub session_id: String,
    /// Current session status.
    pub status: Status,
    /// Elapsed seconds in the current status.
    pub elapsed_seconds: u64,
}

impl SessionUpdate {
    /// Creates a new SessionUpdate with the specified parameters.
    pub fn new(session_id: String, status: Status, elapsed_seconds: u64) -> Self {
        Self {
            session_id,
            status,
            elapsed_seconds,
        }
    }
}
