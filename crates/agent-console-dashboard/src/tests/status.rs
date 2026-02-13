//! Tests for Status enum and related functionality.

use crate::*;

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
