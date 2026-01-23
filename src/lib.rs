//! Agent Console Dashboard library
//!
//! This crate provides the core functionality for the Agent Console daemon,
//! including daemon process management and configuration.

use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Daemon module providing process lifecycle management and daemonization.
pub mod daemon;

/// Session status enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Agent type enumeration representing different AI coding agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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
    pub id: String,
    /// Type of agent (ClaudeCode, etc.).
    pub agent_type: AgentType,
    /// Current session status.
    pub status: Status,
    /// Working directory for this session.
    pub working_dir: PathBuf,
    /// Timestamp when status last changed.
    pub since: Instant,
    /// History of state transitions (display limited by dashboard, not enforced here).
    pub history: Vec<StateTransition>,
    /// Optional API usage tracking.
    pub api_usage: Option<ApiUsage>,
    /// Whether session has been closed (for resurrection).
    pub closed: bool,
    /// Claude Code session ID for resume capability.
    pub session_id: Option<String>,
}

impl Session {
    /// Creates a new Session with the specified parameters.
    pub fn new(id: String, agent_type: AgentType, working_dir: PathBuf) -> Self {
        Self {
            id,
            agent_type,
            status: Status::Working,
            working_dir,
            since: Instant::now(),
            history: Vec::new(),
            api_usage: None,
            closed: false,
            session_id: None,
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new(String::new(), AgentType::ClaudeCode, PathBuf::new())
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
            socket_path: PathBuf::from("/tmp/agent-console.sock"),
            daemonize: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_config_default() {
        let config = DaemonConfig::default();
        assert_eq!(config.socket_path, PathBuf::from("/tmp/agent-console.sock"));
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
            PathBuf::from("/home/user/project"),
        );
        assert_eq!(session.id, "test-session-1");
        assert_eq!(session.agent_type, AgentType::ClaudeCode);
        assert_eq!(session.status, Status::Working);
        assert_eq!(session.working_dir, PathBuf::from("/home/user/project"));
        assert!(session.history.is_empty());
        assert!(session.api_usage.is_none());
        assert!(!session.closed);
        assert!(session.session_id.is_none());
    }

    #[test]
    fn test_session_default() {
        let session = Session::default();
        assert_eq!(session.id, "");
        assert_eq!(session.agent_type, AgentType::ClaudeCode);
        assert_eq!(session.status, Status::Working);
        assert_eq!(session.working_dir, PathBuf::new());
        assert!(session.history.is_empty());
        assert!(session.api_usage.is_none());
        assert!(!session.closed);
        assert!(session.session_id.is_none());
    }

    #[test]
    fn test_session_clone() {
        let session = Session::new(
            "clone-test".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/tmp/test"),
        );
        let cloned = session.clone();
        assert_eq!(cloned.id, session.id);
        assert_eq!(cloned.agent_type, session.agent_type);
        assert_eq!(cloned.status, session.status);
        assert_eq!(cloned.working_dir, session.working_dir);
    }

    #[test]
    fn test_session_with_all_fields() {
        let mut session = Session::new(
            "full-session".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/home/user/project"),
        );
        session.status = Status::Question;
        session.api_usage = Some(ApiUsage {
            input_tokens: 1000,
            output_tokens: 500,
        });
        session.closed = true;
        session.session_id = Some("claude-session-123".to_string());
        session.history.push(StateTransition {
            timestamp: Instant::now(),
            from: Status::Working,
            to: Status::Question,
            duration: Duration::from_secs(60),
        });

        assert_eq!(session.id, "full-session");
        assert_eq!(session.status, Status::Question);
        assert!(session.closed);
        assert_eq!(session.session_id, Some("claude-session-123".to_string()));
        assert_eq!(session.api_usage.unwrap().input_tokens, 1000);
        assert_eq!(session.history.len(), 1);
    }

    #[test]
    fn test_session_field_mutability() {
        let mut session = Session::default();
        session.id = "updated-id".to_string();
        session.status = Status::Attention;
        session.working_dir = PathBuf::from("/new/path");
        session.closed = true;

        assert_eq!(session.id, "updated-id");
        assert_eq!(session.status, Status::Attention);
        assert_eq!(session.working_dir, PathBuf::from("/new/path"));
        assert!(session.closed);
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
            PathBuf::from("/tmp"),
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
}
