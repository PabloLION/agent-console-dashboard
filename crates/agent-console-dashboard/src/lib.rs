//! Agent Console Dashboard library
//!
//! This crate provides the core functionality for the Agent Console daemon,
//! including daemon process management and configuration.
//!
//! # Platform Support
//!
//! This crate currently supports **Unix-like systems only** (Linux, macOS).
//! Windows support is planned for a future release.
//!
//! Unix-specific features used:
//! - Unix domain sockets for IPC
//! - `fork()` for daemon process creation
//! - Unix signal handling (SIGTERM, SIGINT)

use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, Instant};

/// Configuration utilities including XDG path resolution.
pub mod config;

/// Daemon module providing process lifecycle management and daemonization.
pub mod daemon;

/// Layout system for dashboard widget arrangement.
pub mod layout;

/// TUI module providing the terminal user interface for the dashboard.
pub mod tui;

/// Widget system for composable dashboard UI components.
pub mod widgets;

/// Terminal execution module for running commands in panes/terminals.
pub mod terminal;

/// Integration modules for external tools (Zellij, tmux, etc.).
pub mod integrations;

/// Client module for daemon communication with lazy-start capability.
pub mod client;

/// Duration of inactivity (no hook events) before a session is considered inactive.
/// Used by both the daemon idle timer and the TUI for visual treatment.
pub const INACTIVE_SESSION_THRESHOLD: Duration = Duration::from_secs(3600);

/// Session status enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Status {
    /// Agent is actively working
    Working,
    /// Agent needs attention (error/warning)
    Attention,
    /// Agent is asking a question
    Question,
    /// Session has been closed
    Closed,
}

impl Status {
    /// Returns `true` if this status should be visually dimmed in the TUI.
    ///
    /// Both inactive and closed sessions should appear dimmed to indicate
    /// they are not actively in use.
    pub fn should_dim(self) -> bool {
        matches!(self, Status::Closed)
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Status::Working => "working",
            Status::Attention => "attention",
            Status::Question => "question",
            Status::Closed => "closed",
        };
        write!(f, "{}", s)
    }
}

/// Error type for parsing Status from string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseStatusError(pub String);

impl fmt::Display for ParseStatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid status: {}", self.0)
    }
}

impl std::error::Error for ParseStatusError {}

impl FromStr for Status {
    type Err = ParseStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "working" => Ok(Status::Working),
            "attention" => Ok(Status::Attention),
            "question" => Ok(Status::Question),
            "closed" => Ok(Status::Closed),
            _ => Err(ParseStatusError(s.to_string())),
        }
    }
}

/// Agent type enumeration representing different AI coding agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AgentType {
    /// Claude Code - Anthropic's AI coding assistant
    ClaudeCode,
}

/// Record of a state transition for tracking session history.
#[derive(Debug, Clone)]
pub struct StateTransition {
    /// When the transition occurred.
    pub timestamp: Instant,
    /// Previous status before the transition.
    pub from: Status,
    /// New status after the transition.
    pub to: Status,
    /// Duration spent in the previous status.
    pub duration: Duration,
}

/// API token usage tracking for a session.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ApiUsage {
    /// Number of input tokens consumed.
    pub input_tokens: u64,
    /// Number of output tokens generated.
    pub output_tokens: u64,
}

/// Agent session state with history tracking.
#[derive(Debug, Clone)]
pub struct Session {
    /// Unique session identifier.
    pub session_id: String,
    /// Type of agent (ClaudeCode, etc.).
    pub agent_type: AgentType,
    /// Current session status.
    pub status: Status,
    /// Working directory for this session.
    pub working_dir: Option<PathBuf>,
    /// Timestamp when status last changed.
    pub since: Instant,
    /// Timestamp of last hook activity (updated on every `set_status` call,
    /// even when the status is unchanged). Used for stale session detection.
    pub last_activity: Instant,
    /// History of state transitions (display limited by dashboard, not enforced here).
    pub history: Vec<StateTransition>,
    /// Optional API usage tracking.
    pub api_usage: Option<ApiUsage>,
    /// Whether session has been closed (for resurrection).
    pub closed: bool,
}

impl Session {
    /// Creates a new Session with the specified parameters.
    pub fn new(session_id: String, agent_type: AgentType, working_dir: Option<PathBuf>) -> Self {
        Self {
            session_id,
            agent_type,
            status: Status::Working,
            working_dir,
            since: Instant::now(),
            last_activity: Instant::now(),
            history: Vec::new(),
            api_usage: None,
            closed: false,
        }
    }

    /// Updates the session status, recording a state transition if the status changes.
    ///
    /// Same-status transitions reset the elapsed timer but do not record a state
    /// transition. Different-status transitions append a `StateTransition` to the
    /// history with the duration spent in the previous state.
    ///
    /// # Arguments
    ///
    /// * `new_status` - The new status to set for this session.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_console_dashboard::{Session, Status, AgentType};
    /// use std::path::PathBuf;
    ///
    /// let mut session = Session::new(
    ///     "session-1".to_string(),
    ///     AgentType::ClaudeCode,
    ///     Some(PathBuf::from("/home/user/project")),
    /// );
    /// assert_eq!(session.status, Status::Working);
    /// assert!(session.history.is_empty());
    ///
    /// session.set_status(Status::Attention);
    /// assert_eq!(session.status, Status::Attention);
    /// assert_eq!(session.history.len(), 1);
    /// ```
    pub fn set_status(&mut self, new_status: Status) {
        let now = Instant::now();

        // Always record activity, even if status unchanged (for inactive detection).
        self.last_activity = now;

        // Same status: reset elapsed timer but don't record transition
        if self.status == new_status {
            self.since = now;
            return;
        }
        let duration = now.duration_since(self.since);

        // Record the transition
        let transition = StateTransition {
            timestamp: now,
            from: self.status,
            to: new_status,
            duration,
        };

        self.history.push(transition);

        // Update current status and timestamp
        self.status = new_status;
        self.since = now;

        self.closed = new_status == Status::Closed;
    }

    /// Returns `true` if this session has received no hook activity for longer
    /// than `threshold`. Closed sessions are never considered inactive.
    pub fn is_inactive(&self, threshold: Duration) -> bool {
        !self.closed && self.last_activity.elapsed() > threshold
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new(String::new(), AgentType::ClaudeCode, None)
    }
}

/// Configuration for the daemon process.
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Path to the Unix socket for IPC communication.
    pub socket_path: PathBuf,
    /// Whether to run as a background daemon (detached from terminal).
    pub daemonize: bool,
}

impl DaemonConfig {
    /// Creates a new DaemonConfig with the specified socket path and daemonize flag.
    pub fn new(socket_path: PathBuf, daemonize: bool) -> Self {
        Self {
            socket_path,
            daemonize,
        }
    }
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: PathBuf::from("/tmp/agent-console-dashboard.sock"),
            daemonize: false,
        }
    }
}

/// Errors that can occur during session store operations.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// Attempted to create a session that already exists.
    #[error("Session already exists: {0}")]
    SessionExists(String),

    /// Session was not found in the store.
    #[error("Session not found: {0}")]
    SessionNotFound(String),
}

/// Session count breakdown for health status reporting.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct SessionCounts {
    /// Count of active (non-closed) sessions.
    pub active: usize,
    /// Count of closed sessions.
    pub closed: usize,
}

/// Health status response from the daemon STATUS command.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct HealthStatus {
    /// Daemon uptime in seconds.
    pub uptime_seconds: u64,
    /// Session count breakdown.
    pub sessions: SessionCounts,
    /// Count of active connections to the daemon.
    pub connections: usize,
    /// Process memory usage in MB (None if unavailable).
    pub memory_mb: Option<f64>,
    /// Path to the Unix domain socket.
    pub socket_path: String,
}

/// Full daemon state dump for diagnostics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct DaemonDump {
    /// Daemon uptime in seconds.
    pub uptime_seconds: u64,
    /// Path to the Unix domain socket.
    pub socket_path: String,
    /// Snapshot of all sessions.
    pub sessions: Vec<DumpSession>,
    /// Session count breakdown.
    pub session_counts: SessionCounts,
    /// Count of active connections to the daemon.
    pub connections: usize,
}

/// Summary of a single session for dump output.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct DumpSession {
    /// Unique session identifier.
    pub session_id: String,
    /// Current session status as string.
    pub status: String,
    /// Working directory for this session.
    pub working_dir: Option<String>,
    /// Elapsed seconds in the current status.
    pub elapsed_seconds: u64,
    /// Whether session has been closed.
    pub closed: bool,
}

/// Formats a duration in seconds to a human-readable string.
///
/// Returns "Xh Ym" for durations >= 1 hour, "Xm" otherwise.
pub fn format_uptime(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

/// Queries the current process memory usage via sysinfo.
///
/// Returns the RSS in megabytes, or None if the process cannot be found.
pub fn get_memory_usage_mb() -> Option<f64> {
    use sysinfo::{Pid, System};

    let pid = Pid::from_u32(std::process::id());
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
    sys.process(pid)
        .map(|proc_info| proc_info.memory() as f64 / 1024.0 / 1024.0)
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

// ---------------------------------------------------------------------------
// IPC wire types (JSON Lines protocol)
// ---------------------------------------------------------------------------

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_config_default() {
        let config = DaemonConfig::default();
        assert_eq!(
            config.socket_path,
            PathBuf::from("/tmp/agent-console-dashboard.sock")
        );
        assert!(!config.daemonize);
    }

    #[test]
    fn test_daemon_config_new() {
        let config = DaemonConfig::new(PathBuf::from("/custom/path.sock"), true);
        assert_eq!(config.socket_path, PathBuf::from("/custom/path.sock"));
        assert!(config.daemonize);
    }

    #[test]
    fn test_status_equality() {
        assert_eq!(Status::Working, Status::Working);
        assert_eq!(Status::Attention, Status::Attention);
        assert_eq!(Status::Question, Status::Question);
        assert_eq!(Status::Closed, Status::Closed);
        assert_ne!(Status::Working, Status::Closed);
        assert_ne!(Status::Attention, Status::Question);
    }

    #[test]
    fn test_agent_type_equality() {
        assert_eq!(AgentType::ClaudeCode, AgentType::ClaudeCode);
    }

    #[test]
    fn test_state_transition_creation() {
        let transition = StateTransition {
            timestamp: Instant::now(),
            from: Status::Working,
            to: Status::Question,
            duration: Duration::from_secs(30),
        };
        assert_eq!(transition.from, Status::Working);
        assert_eq!(transition.to, Status::Question);
        assert_eq!(transition.duration, Duration::from_secs(30));
    }

    #[test]
    fn test_state_transition_clone() {
        let transition = StateTransition {
            timestamp: Instant::now(),
            from: Status::Attention,
            to: Status::Closed,
            duration: Duration::from_millis(500),
        };
        let cloned = transition.clone();
        assert_eq!(cloned.from, transition.from);
        assert_eq!(cloned.to, transition.to);
        assert_eq!(cloned.duration, transition.duration);
    }

    #[test]
    fn test_api_usage_default() {
        let usage = ApiUsage::default();
        assert_eq!(usage.input_tokens, 0);
        assert_eq!(usage.output_tokens, 0);
    }

    #[test]
    fn test_api_usage_creation() {
        let usage = ApiUsage {
            input_tokens: 1500,
            output_tokens: 2000,
        };
        assert_eq!(usage.input_tokens, 1500);
        assert_eq!(usage.output_tokens, 2000);
    }

    #[test]
    fn test_api_usage_copy() {
        let usage = ApiUsage {
            input_tokens: 100,
            output_tokens: 200,
        };
        let copied = usage;
        assert_eq!(copied.input_tokens, usage.input_tokens);
        assert_eq!(copied.output_tokens, usage.output_tokens);
    }

    #[test]
    fn test_session_new() {
        let session = Session::new(
            "test-session-1".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/home/user/project")),
        );
        assert_eq!(session.session_id, "test-session-1");
        assert_eq!(session.agent_type, AgentType::ClaudeCode);
        assert_eq!(session.status, Status::Working);
        assert_eq!(
            session.working_dir,
            Some(PathBuf::from("/home/user/project"))
        );
        assert!(session.history.is_empty());
        assert!(session.api_usage.is_none());
        assert!(!session.closed);
    }

    #[test]
    fn test_session_default() {
        let session = Session::default();
        assert_eq!(session.session_id, "");
        assert_eq!(session.agent_type, AgentType::ClaudeCode);
        assert_eq!(session.status, Status::Working);
        assert_eq!(session.working_dir, None);
        assert!(session.history.is_empty());
        assert!(session.api_usage.is_none());
        assert!(!session.closed);
    }

    #[test]
    fn test_session_clone() {
        let session = Session::new(
            "clone-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/test")),
        );
        let cloned = session.clone();
        assert_eq!(cloned.session_id, session.session_id);
        assert_eq!(cloned.agent_type, session.agent_type);
        assert_eq!(cloned.status, session.status);
        assert_eq!(cloned.working_dir, session.working_dir);
    }

    #[test]
    fn test_session_with_all_fields() {
        let mut session = Session::new(
            "full-session".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/home/user/project")),
        );
        session.status = Status::Question;
        session.api_usage = Some(ApiUsage {
            input_tokens: 1000,
            output_tokens: 500,
        });
        session.closed = true;
        session.history.push(StateTransition {
            timestamp: Instant::now(),
            from: Status::Working,
            to: Status::Question,
            duration: Duration::from_secs(60),
        });

        assert_eq!(session.session_id, "full-session");
        assert_eq!(session.status, Status::Question);
        assert!(session.closed);
        assert_eq!(session.api_usage.unwrap().input_tokens, 1000);
        assert_eq!(session.history.len(), 1);
    }

    #[test]
    fn test_session_field_mutability() {
        let mut session = Session::default();
        session.session_id = "updated-id".to_string();
        session.status = Status::Attention;
        session.working_dir = Some(PathBuf::from("/new/path"));
        session.closed = true;

        assert_eq!(session.session_id, "updated-id");
        assert_eq!(session.status, Status::Attention);
        assert_eq!(session.working_dir, Some(PathBuf::from("/new/path")));
        assert!(session.closed);
    }

    #[test]
    fn test_session_set_status_changes_status() {
        let mut session = Session::new(
            "status-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
        );
        assert_eq!(session.status, Status::Working);

        session.set_status(Status::Attention);
        assert_eq!(session.status, Status::Attention);
    }

    #[test]
    fn test_session_set_status_records_transition() {
        let mut session = Session::new(
            "transition-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
        );
        assert!(session.history.is_empty());

        session.set_status(Status::Question);

        assert_eq!(session.history.len(), 1);
        assert_eq!(session.history[0].from, Status::Working);
        assert_eq!(session.history[0].to, Status::Question);
    }

    #[test]
    fn test_session_set_status_same_status_no_transition() {
        let mut session = Session::new(
            "same-status-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
        );

        // Setting to the same status should not record a transition
        session.set_status(Status::Working);
        assert!(session.history.is_empty());
        assert_eq!(session.status, Status::Working);
    }

    #[test]
    fn test_session_set_status_same_status_resets_since() {
        let mut session = Session::new(
            "same-status-reset-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
        );
        let original_since = session.since;

        // Small delay to ensure time difference
        std::thread::sleep(Duration::from_millis(10));

        // Setting same status should reset 'since'
        session.set_status(Status::Working);

        // History should still be empty (no transition recorded)
        assert!(session.history.is_empty());
        // But 'since' should be reset to a later time
        assert!(
            session.since > original_since,
            "since should be reset on same-status transition"
        );
    }

    #[test]
    fn test_session_set_status_multiple_transitions() {
        let mut session = Session::new(
            "multi-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
        );

        session.set_status(Status::Attention);
        session.set_status(Status::Question);
        session.set_status(Status::Closed);

        assert_eq!(session.history.len(), 3);
        assert_eq!(session.history[0].from, Status::Working);
        assert_eq!(session.history[0].to, Status::Attention);
        assert_eq!(session.history[1].from, Status::Attention);
        assert_eq!(session.history[1].to, Status::Question);
        assert_eq!(session.history[2].from, Status::Question);
        assert_eq!(session.history[2].to, Status::Closed);
    }

    #[test]
    fn test_session_set_status_closed_to_working_clears_closed_flag() {
        let mut session = Session::new(
            "latch-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
        );

        // Close the session
        session.set_status(Status::Closed);
        assert!(session.closed, "closed flag should be set");

        // Re-activate the session
        session.set_status(Status::Working);
        assert!(
            !session.closed,
            "closed flag should be cleared on re-activation"
        );
        assert_eq!(session.status, Status::Working);
    }

    #[test]
    fn test_session_set_status_updates_since() {
        let mut session = Session::new(
            "since-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
        );
        let original_since = session.since;

        // Small delay to ensure time difference
        std::thread::sleep(Duration::from_millis(10));

        session.set_status(Status::Attention);

        // 'since' should be updated to a later time
        assert!(session.since > original_since);
    }

    #[test]
    fn test_session_set_status_updates_last_activity_on_change() {
        let mut session = Session::new(
            "activity-change-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
        );
        let original = session.last_activity;

        std::thread::sleep(Duration::from_millis(10));
        session.set_status(Status::Attention);

        assert!(session.last_activity > original);
    }

    #[test]
    fn test_session_set_status_updates_last_activity_on_same_status() {
        let mut session = Session::new(
            "activity-same-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
        );
        let original_activity = session.last_activity;
        let original_since = session.since;

        std::thread::sleep(Duration::from_millis(10));

        // Same status: both last_activity and since should advance (timer reset)
        session.set_status(Status::Working);

        assert!(
            session.last_activity > original_activity,
            "last_activity should advance on same-status call"
        );
        assert!(
            session.since > original_since,
            "since should advance on same-status call (timer reset)"
        );
    }

    #[test]
    fn test_session_is_inactive_when_old() {
        let mut session = Session::new(
            "inactive-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
        );
        let threshold = Duration::from_secs(3600);

        // Fresh session is not inactive
        assert!(!session.is_inactive(threshold));

        // Backdate last_activity to 2 hours ago
        session.last_activity = session
            .last_activity
            .checked_sub(Duration::from_secs(7200))
            .expect("backdate should succeed");
        assert!(session.is_inactive(threshold));
    }

    #[test]
    fn test_session_is_inactive_excludes_closed() {
        let mut session = Session::new(
            "closed-inactive-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
        );
        let threshold = Duration::from_secs(3600);

        // Backdate and close
        session.last_activity = session
            .last_activity
            .checked_sub(Duration::from_secs(7200))
            .expect("backdate should succeed");
        session.set_status(Status::Closed);

        assert!(
            !session.is_inactive(threshold),
            "closed sessions are never inactive"
        );
    }

    #[test]
    fn test_session_set_status_transition_has_duration() {
        let mut session = Session::new(
            "duration-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
        );

        // Small delay to ensure measurable duration
        std::thread::sleep(Duration::from_millis(10));

        session.set_status(Status::Question);

        assert_eq!(session.history.len(), 1);
        // Duration should be at least 10ms
        assert!(session.history[0].duration >= Duration::from_millis(10));
    }

    #[test]
    fn test_status_copy() {
        let status = Status::Working;
        let copied = status;
        // After copy, original should still be usable (Copy trait)
        assert_eq!(status, Status::Working);
        assert_eq!(copied, Status::Working);
    }

    #[test]
    fn test_status_all_variants() {
        let statuses = [
            Status::Working,
            Status::Attention,
            Status::Question,
            Status::Closed,
        ];
        // Verify all variants are distinct
        for (i, s1) in statuses.iter().enumerate() {
            for (j, s2) in statuses.iter().enumerate() {
                if i == j {
                    assert_eq!(s1, s2);
                } else {
                    assert_ne!(s1, s2);
                }
            }
        }
    }

    #[test]
    fn test_status_should_dim_closed() {
        assert!(
            Status::Closed.should_dim(),
            "Closed status should be dimmed"
        );
    }

    #[test]
    fn test_status_should_dim_working() {
        assert!(
            !Status::Working.should_dim(),
            "Working status should not be dimmed"
        );
    }

    #[test]
    fn test_status_should_dim_attention() {
        assert!(
            !Status::Attention.should_dim(),
            "Attention status should not be dimmed"
        );
    }

    #[test]
    fn test_status_should_dim_question() {
        assert!(
            !Status::Question.should_dim(),
            "Question status should not be dimmed"
        );
    }

    #[test]
    fn test_agent_type_copy() {
        let agent = AgentType::ClaudeCode;
        let copied = agent;
        // After copy, original should still be usable (Copy trait)
        assert_eq!(agent, AgentType::ClaudeCode);
        assert_eq!(copied, AgentType::ClaudeCode);
    }

    #[test]
    fn test_api_usage_equality() {
        let usage1 = ApiUsage {
            input_tokens: 100,
            output_tokens: 200,
        };
        let usage2 = ApiUsage {
            input_tokens: 100,
            output_tokens: 200,
        };
        let usage3 = ApiUsage {
            input_tokens: 100,
            output_tokens: 300,
        };
        assert_eq!(usage1, usage2);
        assert_ne!(usage1, usage3);
    }

    #[test]
    fn test_state_transition_all_status_variants() {
        // Test StateTransition with various status combinations
        let transitions = vec![
            (Status::Working, Status::Question),
            (Status::Working, Status::Attention),
            (Status::Question, Status::Working),
            (Status::Attention, Status::Closed),
            (Status::Working, Status::Closed),
        ];

        for (from, to) in transitions {
            let transition = StateTransition {
                timestamp: Instant::now(),
                from,
                to,
                duration: Duration::from_millis(100),
            };
            assert_eq!(transition.from, from);
            assert_eq!(transition.to, to);
        }
    }

    #[test]
    fn test_session_history_multiple_entries() {
        let mut session = Session::default();

        // Add multiple history entries
        for i in 0..5 {
            session.history.push(StateTransition {
                timestamp: Instant::now(),
                from: Status::Working,
                to: Status::Question,
                duration: Duration::from_secs(i as u64),
            });
        }

        assert_eq!(session.history.len(), 5);
        assert_eq!(session.history[0].duration, Duration::from_secs(0));
        assert_eq!(session.history[4].duration, Duration::from_secs(4));
    }

    #[test]
    fn test_session_debug_format() {
        let session = Session::new(
            "debug-test".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
        );
        let debug_str = format!("{:?}", session);
        // Debug output should contain the session ID
        assert!(debug_str.contains("debug-test"));
        assert!(debug_str.contains("ClaudeCode"));
    }

    #[test]
    fn test_status_debug_format() {
        assert_eq!(format!("{:?}", Status::Working), "Working");
        assert_eq!(format!("{:?}", Status::Attention), "Attention");
        assert_eq!(format!("{:?}", Status::Question), "Question");
        assert_eq!(format!("{:?}", Status::Closed), "Closed");
    }

    #[test]
    fn test_agent_type_debug_format() {
        assert_eq!(format!("{:?}", AgentType::ClaudeCode), "ClaudeCode");
    }

    #[test]
    fn test_api_usage_debug_format() {
        let usage = ApiUsage {
            input_tokens: 42,
            output_tokens: 84,
        };
        let debug_str = format!("{:?}", usage);
        assert!(debug_str.contains("42"));
        assert!(debug_str.contains("84"));
    }

    #[test]
    fn test_store_error_session_exists() {
        let error = StoreError::SessionExists("test-session".to_string());
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("Session already exists"));
        assert!(error_msg.contains("test-session"));
    }

    #[test]
    fn test_store_error_session_not_found() {
        let error = StoreError::SessionNotFound("missing-session".to_string());
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("Session not found"));
        assert!(error_msg.contains("missing-session"));
    }

    #[test]
    fn test_store_error_debug_format() {
        let error = StoreError::SessionExists("debug-test".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("SessionExists"));
        assert!(debug_str.contains("debug-test"));
    }

    #[test]
    fn test_store_error_is_std_error() {
        let error: Box<dyn std::error::Error> =
            Box::new(StoreError::SessionNotFound("test".to_string()));
        // Verify it can be used as a std::error::Error
        assert!(error.to_string().contains("Session not found"));
    }

    #[test]
    fn test_session_metadata_default() {
        let metadata = SessionMetadata::default();
        assert!(metadata.working_dir.is_none());
        assert!(metadata.session_id.is_none());
        assert!(metadata.agent_type.is_none());
    }

    #[test]
    fn test_session_metadata_new() {
        let metadata = SessionMetadata::new();
        assert!(metadata.working_dir.is_none());
        assert!(metadata.session_id.is_none());
        assert!(metadata.agent_type.is_none());
    }

    #[test]
    fn test_session_metadata_with_working_dir() {
        let metadata = SessionMetadata::with_working_dir(PathBuf::from("/home/user/project"));
        assert_eq!(
            metadata.working_dir,
            Some(PathBuf::from("/home/user/project"))
        );
        assert!(metadata.session_id.is_none());
        assert!(metadata.agent_type.is_none());
    }

    #[test]
    fn test_session_metadata_builder_pattern() {
        let metadata = SessionMetadata::new()
            .working_dir(PathBuf::from("/tmp/test"))
            .session_id("session-123".to_string())
            .agent_type(AgentType::ClaudeCode);

        assert_eq!(metadata.working_dir, Some(PathBuf::from("/tmp/test")));
        assert_eq!(metadata.session_id, Some("session-123".to_string()));
        assert_eq!(metadata.agent_type, Some(AgentType::ClaudeCode));
    }

    #[test]
    fn test_session_metadata_clone() {
        let metadata = SessionMetadata::new()
            .working_dir(PathBuf::from("/clone/test"))
            .session_id("clone-session".to_string());

        let cloned = metadata.clone();
        assert_eq!(cloned.working_dir, metadata.working_dir);
        assert_eq!(cloned.session_id, metadata.session_id);
        assert_eq!(cloned.agent_type, metadata.agent_type);
    }

    #[test]
    fn test_session_metadata_debug_format() {
        let metadata = SessionMetadata::new()
            .working_dir(PathBuf::from("/debug/path"))
            .session_id("debug-session".to_string());

        let debug_str = format!("{:?}", metadata);
        assert!(debug_str.contains("SessionMetadata"));
        assert!(debug_str.contains("/debug/path"));
        assert!(debug_str.contains("debug-session"));
    }

    #[test]
    fn test_session_metadata_partial_fields() {
        // Test with only working_dir
        let metadata1 = SessionMetadata {
            working_dir: Some(PathBuf::from("/only/working/dir")),
            session_id: None,
            agent_type: None,
        };
        assert!(metadata1.working_dir.is_some());
        assert!(metadata1.session_id.is_none());

        // Test with only session_id
        let metadata2 = SessionMetadata {
            working_dir: None,
            session_id: Some("only-session-id".to_string()),
            agent_type: None,
        };
        assert!(metadata2.working_dir.is_none());
        assert!(metadata2.session_id.is_some());

        // Test with only agent_type
        let metadata3 = SessionMetadata {
            working_dir: None,
            session_id: None,
            agent_type: Some(AgentType::ClaudeCode),
        };
        assert!(metadata3.working_dir.is_none());
        assert!(metadata3.agent_type.is_some());
    }

    #[test]
    fn test_session_update_new() {
        let update = SessionUpdate::new("session-1".to_string(), Status::Working, 120);
        assert_eq!(update.session_id, "session-1");
        assert_eq!(update.status, Status::Working);
        assert_eq!(update.elapsed_seconds, 120);
    }

    #[test]
    fn test_session_update_clone() {
        let update = SessionUpdate::new("clone-test".to_string(), Status::Attention, 60);
        let cloned = update.clone();
        assert_eq!(cloned.session_id, update.session_id);
        assert_eq!(cloned.status, update.status);
        assert_eq!(cloned.elapsed_seconds, update.elapsed_seconds);
    }

    #[test]
    fn test_session_update_equality() {
        let update1 = SessionUpdate::new("eq-test".to_string(), Status::Question, 30);
        let update2 = SessionUpdate::new("eq-test".to_string(), Status::Question, 30);
        let update3 = SessionUpdate::new("eq-test".to_string(), Status::Working, 30);
        assert_eq!(update1, update2);
        assert_ne!(update1, update3);
    }

    #[test]
    fn test_session_update_debug_format() {
        let update = SessionUpdate::new("debug-test".to_string(), Status::Closed, 45);
        let debug_str = format!("{:?}", update);
        assert!(debug_str.contains("debug-test"));
        assert!(debug_str.contains("Closed"));
        assert!(debug_str.contains("45"));
    }

    #[test]
    fn test_session_update_all_statuses() {
        for status in [
            Status::Working,
            Status::Attention,
            Status::Question,
            Status::Closed,
        ] {
            let update = SessionUpdate::new("status-test".to_string(), status, 0);
            assert_eq!(update.status, status);
        }
    }

    #[test]
    fn test_format_uptime_minutes_only() {
        assert_eq!(format_uptime(0), "0m");
        assert_eq!(format_uptime(59), "0m");
        assert_eq!(format_uptime(60), "1m");
        assert_eq!(format_uptime(600), "10m");
        assert_eq!(format_uptime(3599), "59m");
    }

    #[test]
    fn test_format_uptime_hours_and_minutes() {
        assert_eq!(format_uptime(3600), "1h 0m");
        assert_eq!(format_uptime(3660), "1h 1m");
        assert_eq!(format_uptime(9240), "2h 34m");
        assert_eq!(format_uptime(86400), "24h 0m");
    }

    #[test]
    fn test_health_status_serialization_roundtrip() {
        let health = HealthStatus {
            uptime_seconds: 9240,
            sessions: SessionCounts {
                active: 3,
                closed: 1,
            },
            connections: 2,
            memory_mb: Some(2.1),
            socket_path: "/tmp/acd.sock".to_string(),
        };

        let json = serde_json::to_string(&health).expect("failed to serialize HealthStatus");
        let parsed: HealthStatus =
            serde_json::from_str(&json).expect("failed to deserialize HealthStatus");

        assert_eq!(parsed.uptime_seconds, 9240);
        assert_eq!(parsed.sessions.active, 3);
        assert_eq!(parsed.sessions.closed, 1);
        assert_eq!(parsed.connections, 2);
        assert_eq!(parsed.memory_mb, Some(2.1));
        assert_eq!(parsed.socket_path, "/tmp/acd.sock");
    }

    #[test]
    fn test_health_status_memory_none() {
        let health = HealthStatus {
            uptime_seconds: 0,
            sessions: SessionCounts {
                active: 0,
                closed: 0,
            },
            connections: 0,
            memory_mb: None,
            socket_path: "/tmp/test.sock".to_string(),
        };

        let json = serde_json::to_string(&health).expect("failed to serialize HealthStatus");
        assert!(json.contains("\"memory_mb\":null"));

        let parsed: HealthStatus =
            serde_json::from_str(&json).expect("failed to deserialize HealthStatus");
        assert!(parsed.memory_mb.is_none());
    }

    #[test]
    fn test_session_counts_equality() {
        let a = SessionCounts {
            active: 3,
            closed: 1,
        };
        let b = SessionCounts {
            active: 3,
            closed: 1,
        };
        let c = SessionCounts {
            active: 2,
            closed: 1,
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_get_memory_usage_mb_returns_value() {
        // Best-effort test: on most systems this should return Some
        let mem = get_memory_usage_mb();
        // We just verify it doesn't panic; the value may be None in some CI environments
        if let Some(mb) = mem {
            assert!(mb > 0.0, "memory usage should be positive");
        }
    }

    #[test]
    fn test_daemon_dump_serialization_roundtrip() {
        let dump = DaemonDump {
            uptime_seconds: 3600,
            socket_path: "/tmp/test.sock".to_string(),
            sessions: vec![DumpSession {
                session_id: "session-1".to_string(),
                status: "working".to_string(),
                working_dir: Some("/home/user/project".to_string()),
                elapsed_seconds: 120,
                closed: false,
            }],
            session_counts: SessionCounts {
                active: 1,
                closed: 0,
            },
            connections: 2,
        };

        let json = serde_json::to_string(&dump).expect("failed to serialize DaemonDump");
        let parsed: DaemonDump =
            serde_json::from_str(&json).expect("failed to deserialize DaemonDump");
        assert_eq!(parsed, dump);
    }

    #[test]
    fn test_dump_session_serialization() {
        let entry = DumpSession {
            session_id: "snap-1".to_string(),
            status: "attention".to_string(),
            working_dir: Some("/tmp/work".to_string()),
            elapsed_seconds: 45,
            closed: true,
        };

        let json = serde_json::to_string(&entry).expect("failed to serialize DumpSession");
        let parsed: DumpSession =
            serde_json::from_str(&json).expect("failed to deserialize DumpSession");
        assert_eq!(parsed, entry);
    }

    #[test]
    fn test_daemon_dump_empty_sessions() {
        let dump = DaemonDump {
            uptime_seconds: 0,
            socket_path: "/tmp/empty.sock".to_string(),
            sessions: vec![],
            session_counts: SessionCounts {
                active: 0,
                closed: 0,
            },
            connections: 0,
        };

        let json = serde_json::to_string(&dump).expect("failed to serialize DaemonDump");
        let parsed: DaemonDump =
            serde_json::from_str(&json).expect("failed to deserialize DaemonDump");
        assert_eq!(parsed.sessions.len(), 0);
        assert_eq!(parsed.session_counts.active, 0);
    }

    #[test]
    fn test_daemon_dump_multiple_sessions() {
        let dump = DaemonDump {
            uptime_seconds: 7200,
            socket_path: "/tmp/multi.sock".to_string(),
            sessions: vec![
                DumpSession {
                    session_id: "s1".to_string(),
                    status: "working".to_string(),
                    working_dir: Some("/project-a".to_string()),
                    elapsed_seconds: 60,
                    closed: false,
                },
                DumpSession {
                    session_id: "s2".to_string(),
                    status: "closed".to_string(),
                    working_dir: Some("/project-b".to_string()),
                    elapsed_seconds: 300,
                    closed: true,
                },
                DumpSession {
                    session_id: "s3".to_string(),
                    status: "question".to_string(),
                    working_dir: Some("/project-c".to_string()),
                    elapsed_seconds: 10,
                    closed: false,
                },
            ],
            session_counts: SessionCounts {
                active: 2,
                closed: 1,
            },
            connections: 3,
        };

        let json = serde_json::to_string(&dump).expect("failed to serialize DaemonDump");
        let parsed: DaemonDump =
            serde_json::from_str(&json).expect("failed to deserialize DaemonDump");
        assert_eq!(parsed.sessions.len(), 3);
        assert_eq!(parsed.session_counts.active, 2);
        assert_eq!(parsed.session_counts.closed, 1);
        assert_eq!(parsed.connections, 3);
    }
}
