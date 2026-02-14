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

/// IPC wire types for JSON Lines protocol.
mod ipc;
pub use ipc::*;

/// Health status and diagnostics types.
mod health;
pub use health::*;

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

    /// Returns the sort group for this status.
    ///
    /// Lower values sort first. Note that "inactive" is NOT a status variant;
    /// it's determined at sort time by checking `session.is_inactive(threshold)`.
    ///
    /// Sort groups:
    /// - 0: Attention (highest priority)
    /// - 1: Working
    /// - 2: Question
    /// - 3: Closed (lowest priority)
    ///
    /// Inactive sessions (non-closed sessions with idle_seconds > threshold)
    /// are assigned group 2 at sort time.
    pub fn status_group(self) -> u8 {
        match self {
            Status::Attention => 0,
            Status::Working => 1,
            Status::Question => 2,
            Status::Closed => 3,
        }
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
    /// Session priority for sorting (higher = ranked higher).
    pub priority: u64,
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
            priority: 0,
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
        Self {
            session_id: String::new(),
            agent_type: AgentType::ClaudeCode,
            status: Status::Working,
            working_dir: None,
            since: Instant::now(),
            last_activity: Instant::now(),
            history: Vec::new(),
            api_usage: None,
            closed: false,
            priority: 0,
        }
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

#[cfg(test)]
mod tests;
