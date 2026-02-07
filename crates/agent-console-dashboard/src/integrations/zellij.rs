use crate::terminal::{execute_in_terminal, ExecutionResult, TerminalEnvironment, TerminalError};
use std::path::Path;

/// Result of a resurrection attempt.
#[derive(Debug)]
pub enum ResurrectionResult {
    /// Session is resuming in a new pane.
    ExecutedInPane,
    /// Not in a multiplexer; user should run this command manually.
    ManualCommand(String),
    /// Resurrection failed.
    Failed(String),
}

/// Attempt to resurrect a closed session.
///
/// In Zellij: creates a new pane with `claude --resume <session_id>`.
/// Outside Zellij: returns the command string for user to run manually.
pub fn resurrect_session(
    session_id: &str,
    working_dir: &Path,
) -> Result<ResurrectionResult, TerminalError> {
    let args = vec!["--resume".to_string(), session_id.to_string()];

    match execute_in_terminal("claude", &args, Some(working_dir))? {
        ExecutionResult::Executed => Ok(ResurrectionResult::ExecutedInPane),
        ExecutionResult::DisplayCommand(cmd) => Ok(ResurrectionResult::ManualCommand(cmd)),
        ExecutionResult::Failed(err) => Ok(ResurrectionResult::Failed(err)),
    }
}

/// Check if running inside a terminal multiplexer that supports pane creation.
pub fn can_create_pane() -> bool {
    matches!(TerminalEnvironment::detect(), TerminalEnvironment::Zellij)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_create_pane_returns_false_in_plain_terminal() {
        // Save original env
        let original_zellij = std::env::var("ZELLIJ");
        let original_tmux = std::env::var("TMUX");

        // Set up Plain environment
        std::env::remove_var("ZELLIJ");
        std::env::remove_var("TMUX");

        assert!(!can_create_pane());

        // Restore original env
        if let Ok(val) = original_zellij {
            std::env::set_var("ZELLIJ", val);
        }
        if let Ok(val) = original_tmux {
            std::env::set_var("TMUX", val);
        }
    }

    #[test]
    fn test_resurrection_result_debug_formatting() {
        let executed = ResurrectionResult::ExecutedInPane;
        let debug_str = format!("{:?}", executed);
        assert!(debug_str.contains("ExecutedInPane"));

        let manual = ResurrectionResult::ManualCommand("test command".to_string());
        let debug_str = format!("{:?}", manual);
        assert!(debug_str.contains("ManualCommand"));
        assert!(debug_str.contains("test command"));

        let failed = ResurrectionResult::Failed("error".to_string());
        let debug_str = format!("{:?}", failed);
        assert!(debug_str.contains("Failed"));
        assert!(debug_str.contains("error"));
    }

    #[test]
    fn test_resurrect_session_in_plain_terminal_returns_manual_command() {
        // Save original env
        let original_zellij = std::env::var("ZELLIJ");
        let original_tmux = std::env::var("TMUX");

        // Set up Plain environment
        std::env::remove_var("ZELLIJ");
        std::env::remove_var("TMUX");

        let result = resurrect_session("session-123", Path::new("/home/user/project"))
            .expect("should not fail");

        match result {
            ResurrectionResult::ManualCommand(cmd) => {
                assert_eq!(cmd, "cd /home/user/project && claude --resume session-123");
            }
            _ => panic!("Expected ManualCommand for plain terminal"),
        }

        // Restore original env
        if let Ok(val) = original_zellij {
            std::env::set_var("ZELLIJ", val);
        }
        if let Ok(val) = original_tmux {
            std::env::set_var("TMUX", val);
        }
    }
}
