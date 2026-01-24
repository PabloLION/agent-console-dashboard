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

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Custom serialization module for `std::time::Instant` as Unix timestamp milliseconds.
///
/// Since `Instant` is a monotonic clock that doesn't correspond to wall-clock time,
/// this module converts to/from Unix timestamp milliseconds for IPC serialization.
/// The conversion works by calculating the offset between the Instant and the current
/// time, then applying that offset to the current SystemTime.
///
/// Note: This has minor precision limitations (~millisecond accuracy) but is acceptable
/// for session tracking purposes.
mod serde_instant {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(instant: &Instant, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Convert to milliseconds since UNIX epoch
        let system_now = SystemTime::now();
        let instant_now = Instant::now();
        let elapsed = instant_now.duration_since(*instant);
        let system_time = system_now - elapsed;
        let millis = system_time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        millis.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Instant, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        // Convert milliseconds since UNIX epoch back to Instant
        let system_now = SystemTime::now();
        let now_millis = system_now
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let elapsed = std::time::Duration::from_millis(now_millis.saturating_sub(millis));
        Ok(Instant::now() - elapsed)
    }
}

/// Custom serialization module for `std::time::Duration` as milliseconds.
///
/// This module serializes Duration values as u64 milliseconds for IPC communication.
/// This provides a simple, language-agnostic representation that can be easily
/// consumed by clients.
mod serde_duration {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as milliseconds (u64 for JSON compatibility)
        (duration.as_millis() as u64).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

/// Daemon module providing process lifecycle management and daemonization.
pub mod daemon;

/// Internal client module for daemon communication with auto-start capability.
/// This module is not part of the public API - external tools should use CLI commands.
pub(crate) mod client;

/// Session status enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    /// Claude Code - Anthropic's AI coding assistant
    ClaudeCode,
}

/// Record of a state transition for tracking session history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    /// When the transition occurred.
    #[serde(with = "serde_instant")]
    pub timestamp: Instant,
    /// Previous status before the transition.
    pub from: Status,
    /// New status after the transition.
    pub to: Status,
    /// Duration spent in the previous status.
    #[serde(with = "serde_duration")]
    pub duration: Duration,
}

/// API token usage tracking for a session.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiUsage {
    /// Number of input tokens consumed.
    pub input_tokens: u64,
    /// Number of output tokens generated.
    pub output_tokens: u64,
}

/// Agent session state with history tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(with = "serde_instant")]
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

    #[test]
    fn test_status_serializes_lowercase() {
        // Verify that Status enum serializes to lowercase strings
        assert_eq!(
            serde_json::to_string(&Status::Working).unwrap(),
            "\"working\""
        );
        assert_eq!(
            serde_json::to_string(&Status::Attention).unwrap(),
            "\"attention\""
        );
        assert_eq!(
            serde_json::to_string(&Status::Question).unwrap(),
            "\"question\""
        );
        assert_eq!(
            serde_json::to_string(&Status::Closed).unwrap(),
            "\"closed\""
        );
    }

    #[test]
    fn test_session_json_roundtrip() {
        // Create a Session with all fields populated
        let mut original = Session::new(
            "roundtrip-test-123".to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/home/user/my-project"),
        );
        original.status = Status::Question;
        original.api_usage = Some(ApiUsage {
            input_tokens: 15000,
            output_tokens: 8500,
        });
        original.closed = true;
        original.session_id = Some("claude-sess-abc123".to_string());
        original.history.push(StateTransition {
            timestamp: Instant::now(),
            from: Status::Working,
            to: Status::Question,
            duration: Duration::from_secs(120),
        });

        // Serialize to JSON
        let json = serde_json::to_string(&original).expect("Failed to serialize Session");

        // Deserialize back
        let deserialized: Session =
            serde_json::from_str(&json).expect("Failed to deserialize Session");

        // Verify all fields match
        assert_eq!(deserialized.id, original.id);
        assert_eq!(deserialized.agent_type, original.agent_type);
        assert_eq!(deserialized.status, original.status);
        assert_eq!(deserialized.working_dir, original.working_dir);
        assert_eq!(deserialized.api_usage, original.api_usage);
        assert_eq!(deserialized.closed, original.closed);
        assert_eq!(deserialized.session_id, original.session_id);

        // Verify history was preserved
        assert_eq!(deserialized.history.len(), 1);
        assert_eq!(deserialized.history[0].from, Status::Working);
        assert_eq!(deserialized.history[0].to, Status::Question);
        assert_eq!(deserialized.history[0].duration, Duration::from_secs(120));

        // Verify since timestamp is approximately preserved (within 1 second tolerance
        // due to serialization timing variations)
        let since_diff = if deserialized.since > original.since {
            deserialized.since.duration_since(original.since)
        } else {
            original.since.duration_since(deserialized.since)
        };
        assert!(
            since_diff < Duration::from_secs(1),
            "since timestamp drift too large: {:?}",
            since_diff
        );
    }

    #[test]
    fn test_instant_serializes_as_millis() {
        // Create a wrapper struct to test the serde_instant module
        #[derive(Serialize, Deserialize)]
        struct InstantWrapper {
            #[serde(with = "super::serde_instant")]
            instant: Instant,
        }

        let now = Instant::now();
        let wrapper = InstantWrapper { instant: now };

        // Serialize to JSON
        let json = serde_json::to_string(&wrapper).expect("Failed to serialize Instant");

        // The JSON should contain a number (milliseconds since epoch)
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("Failed to parse JSON");
        assert!(
            value["instant"].is_number(),
            "Expected instant to be serialized as a number (milliseconds)"
        );

        // The value should be a reasonable Unix timestamp in milliseconds
        // (greater than year 2020 timestamp: 1577836800000)
        let millis = value["instant"].as_u64().expect("Failed to get millis as u64");
        assert!(
            millis > 1577836800000,
            "Timestamp should be a Unix timestamp in milliseconds (after 2020)"
        );
    }

    #[test]
    fn test_instant_roundtrip() {
        #[derive(Serialize, Deserialize)]
        struct InstantWrapper {
            #[serde(with = "super::serde_instant")]
            instant: Instant,
        }

        let original = Instant::now();
        let wrapper = InstantWrapper { instant: original };

        // Serialize and deserialize
        let json = serde_json::to_string(&wrapper).expect("Failed to serialize");
        let deserialized: InstantWrapper =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // The deserialized instant should be approximately equal to the original
        // (within 1 second tolerance due to timing variations)
        let diff = if deserialized.instant > original {
            deserialized.instant.duration_since(original)
        } else {
            original.duration_since(deserialized.instant)
        };
        assert!(
            diff < Duration::from_secs(1),
            "Instant roundtrip drift too large: {:?}",
            diff
        );
    }

    #[test]
    fn test_instant_past_serialization() {
        #[derive(Serialize, Deserialize)]
        struct InstantWrapper {
            #[serde(with = "super::serde_instant")]
            instant: Instant,
        }

        // Test with an instant from the past (10 seconds ago)
        let past_instant = Instant::now() - Duration::from_secs(10);
        let wrapper = InstantWrapper {
            instant: past_instant,
        };

        // Serialize and deserialize
        let json = serde_json::to_string(&wrapper).expect("Failed to serialize");
        let deserialized: InstantWrapper =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify the elapsed time is approximately preserved
        let original_elapsed = past_instant.elapsed();
        let deserialized_elapsed = deserialized.instant.elapsed();

        // Both should show approximately 10 seconds elapsed (within 2 second tolerance)
        assert!(
            original_elapsed.as_secs() >= 10,
            "Original elapsed should be at least 10 seconds"
        );
        assert!(
            deserialized_elapsed.as_secs() >= 9 && deserialized_elapsed.as_secs() <= 12,
            "Deserialized elapsed should be approximately 10 seconds, got: {:?}",
            deserialized_elapsed
        );
    }

    #[test]
    fn test_duration_serializes_as_millis() {
        #[derive(Serialize, Deserialize)]
        struct DurationWrapper {
            #[serde(with = "super::serde_duration")]
            duration: Duration,
        }

        let wrapper = DurationWrapper {
            duration: Duration::from_secs(5),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&wrapper).expect("Failed to serialize Duration");

        // The JSON should contain 5000 (milliseconds)
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("Failed to parse JSON");
        assert_eq!(
            value["duration"].as_u64(),
            Some(5000),
            "5 seconds should serialize as 5000 milliseconds"
        );
    }

    #[test]
    fn test_duration_roundtrip() {
        #[derive(Serialize, Deserialize)]
        struct DurationWrapper {
            #[serde(with = "super::serde_duration")]
            duration: Duration,
        }

        let original = Duration::from_millis(12345);
        let wrapper = DurationWrapper { duration: original };

        // Serialize and deserialize
        let json = serde_json::to_string(&wrapper).expect("Failed to serialize");
        let deserialized: DurationWrapper =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(
            deserialized.duration, original,
            "Duration roundtrip should preserve exact value"
        );
    }

    #[test]
    fn test_duration_edge_cases() {
        #[derive(Serialize, Deserialize)]
        struct DurationWrapper {
            #[serde(with = "super::serde_duration")]
            duration: Duration,
        }

        // Test zero duration
        let zero_wrapper = DurationWrapper {
            duration: Duration::ZERO,
        };
        let json = serde_json::to_string(&zero_wrapper).expect("Failed to serialize zero");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["duration"].as_u64(), Some(0));

        // Test sub-millisecond duration (should truncate to 0)
        let sub_ms_wrapper = DurationWrapper {
            duration: Duration::from_micros(500),
        };
        let json = serde_json::to_string(&sub_ms_wrapper).expect("Failed to serialize sub-ms");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            value["duration"].as_u64(),
            Some(0),
            "Sub-millisecond duration should truncate to 0"
        );

        // Test large duration (1 day)
        let day_wrapper = DurationWrapper {
            duration: Duration::from_secs(86400),
        };
        let json = serde_json::to_string(&day_wrapper).expect("Failed to serialize day");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            value["duration"].as_u64(),
            Some(86400000),
            "1 day should be 86400000 milliseconds"
        );
    }

    #[test]
    fn test_state_transition_serialization() {
        // Test the full StateTransition struct which uses both serde_instant and serde_duration
        let transition = StateTransition {
            timestamp: Instant::now(),
            from: Status::Working,
            to: Status::Question,
            duration: Duration::from_secs(30),
        };

        // Serialize
        let json = serde_json::to_string(&transition).expect("Failed to serialize StateTransition");

        // Parse and verify structure
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        // timestamp should be a number (millis since epoch)
        assert!(
            value["timestamp"].is_number(),
            "timestamp should be serialized as number"
        );

        // duration should be 30000 milliseconds
        assert_eq!(
            value["duration"].as_u64(),
            Some(30000),
            "duration should be 30000 ms"
        );

        // from/to should be lowercase strings
        assert_eq!(value["from"].as_str(), Some("working"));
        assert_eq!(value["to"].as_str(), Some("question"));

        // Deserialize and verify
        let deserialized: StateTransition =
            serde_json::from_str(&json).expect("Failed to deserialize StateTransition");
        assert_eq!(deserialized.from, Status::Working);
        assert_eq!(deserialized.to, Status::Question);
        assert_eq!(deserialized.duration, Duration::from_secs(30));
    }
}
