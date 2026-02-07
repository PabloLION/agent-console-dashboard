use super::environment::TerminalEnvironment;
use std::path::Path;
use std::process::Command;

#[derive(Debug)]
pub enum ExecutionResult {
    /// Command was executed in a new pane
    Executed,
    /// Command should be run manually by user
    DisplayCommand(String),
    /// Execution failed
    Failed(String),
}

#[derive(Debug)]
pub enum TerminalError {
    /// Failed to execute the command
    ExecutionFailed(String),
}

impl std::fmt::Display for TerminalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExecutionFailed(msg) => write!(f, "Failed to execute command: {}", msg),
        }
    }
}

impl std::error::Error for TerminalError {}

/// Execute a command in the appropriate terminal context.
pub fn execute_in_terminal(
    cmd: &str,
    args: &[String],
    working_dir: Option<&Path>,
) -> Result<ExecutionResult, TerminalError> {
    let env = TerminalEnvironment::detect();
    match env {
        TerminalEnvironment::Zellij => execute_zellij_pane(cmd, args, working_dir),
        TerminalEnvironment::Tmux | TerminalEnvironment::Plain => Ok(
            ExecutionResult::DisplayCommand(build_command_string(cmd, args, working_dir)),
        ),
    }
}

fn execute_zellij_pane(
    cmd: &str,
    args: &[String],
    working_dir: Option<&Path>,
) -> Result<ExecutionResult, TerminalError> {
    let full_cmd = build_command_string(cmd, args, working_dir);

    let status = Command::new("zellij")
        .args(["action", "new-pane", "--"])
        .args(["sh", "-c", &full_cmd])
        .status()
        .map_err(|e| TerminalError::ExecutionFailed(e.to_string()))?;

    if status.success() {
        Ok(ExecutionResult::Executed)
    } else {
        Ok(ExecutionResult::Failed(
            "Zellij pane creation failed".to_string(),
        ))
    }
}

fn build_command_string(cmd: &str, args: &[String], working_dir: Option<&Path>) -> String {
    let cmd_with_args = if args.is_empty() {
        cmd.to_string()
    } else {
        format!("{} {}", cmd, args.join(" "))
    };

    if let Some(dir) = working_dir {
        format!("cd {} && {}", dir.display(), cmd_with_args)
    } else {
        cmd_with_args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_command_string_with_working_dir() {
        let result = build_command_string(
            "claude",
            &["--resume".to_string()],
            Some(Path::new("/home/user/project")),
        );
        assert_eq!(result, "cd /home/user/project && claude --resume");
    }

    #[test]
    fn test_build_command_string_without_working_dir() {
        let result = build_command_string("claude", &["--resume".to_string()], None);
        assert_eq!(result, "claude --resume");
    }

    #[test]
    fn test_build_command_string_no_args() {
        let result = build_command_string("claude", &[], None);
        assert_eq!(result, "claude");
    }

    #[test]
    fn test_build_command_string_multiple_args() {
        let result = build_command_string(
            "git",
            &[
                "commit".to_string(),
                "-m".to_string(),
                "message".to_string(),
            ],
            None,
        );
        assert_eq!(result, "git commit -m message");
    }

    #[test]
    fn test_build_command_string_with_dir_no_args() {
        let result = build_command_string("pwd", &[], Some(Path::new("/tmp")));
        assert_eq!(result, "cd /tmp && pwd");
    }

    #[test]
    fn test_execution_result_debug() {
        let executed = ExecutionResult::Executed;
        let debug_str = format!("{:?}", executed);
        assert!(debug_str.contains("Executed"));

        let display = ExecutionResult::DisplayCommand("test".to_string());
        let debug_str = format!("{:?}", display);
        assert!(debug_str.contains("DisplayCommand"));

        let failed = ExecutionResult::Failed("error".to_string());
        let debug_str = format!("{:?}", failed);
        assert!(debug_str.contains("Failed"));
    }

    #[test]
    fn test_terminal_error_display() {
        let error = TerminalError::ExecutionFailed("test error".to_string());
        let display_str = format!("{}", error);
        assert_eq!(display_str, "Failed to execute command: test error");
    }

    #[test]
    fn test_terminal_error_debug() {
        let error = TerminalError::ExecutionFailed("debug test".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("ExecutionFailed"));
        assert!(debug_str.contains("debug test"));
    }

    #[test]
    fn test_terminal_error_is_std_error() {
        let error: Box<dyn std::error::Error> =
            Box::new(TerminalError::ExecutionFailed("test".to_string()));
        assert!(error.to_string().contains("Failed to execute command"));
    }

    #[test]
    fn test_execute_in_terminal_tmux_returns_display_command() {
        // Save original env
        let original_tmux = std::env::var("TMUX");
        let original_zellij = std::env::var("ZELLIJ");

        // Set up TMUX environment
        std::env::set_var("TMUX", "1");
        std::env::remove_var("ZELLIJ");

        let result = execute_in_terminal("test", &[], None).expect("should not fail");
        match result {
            ExecutionResult::DisplayCommand(cmd) => {
                assert_eq!(cmd, "test");
            }
            _ => panic!("Expected DisplayCommand for TMUX environment"),
        }

        // Restore original env
        if let Ok(val) = original_tmux {
            std::env::set_var("TMUX", val);
        } else {
            std::env::remove_var("TMUX");
        }
        if let Ok(val) = original_zellij {
            std::env::set_var("ZELLIJ", val);
        } else {
            std::env::remove_var("ZELLIJ");
        }
    }

    #[test]
    fn test_execute_in_terminal_plain_returns_display_command() {
        // Save original env
        let original_tmux = std::env::var("TMUX");
        let original_zellij = std::env::var("ZELLIJ");

        // Set up Plain environment
        std::env::remove_var("TMUX");
        std::env::remove_var("ZELLIJ");

        let result = execute_in_terminal("cmd", &["arg1".to_string()], Some(Path::new("/path")))
            .expect("should not fail");
        match result {
            ExecutionResult::DisplayCommand(cmd) => {
                assert_eq!(cmd, "cd /path && cmd arg1");
            }
            _ => panic!("Expected DisplayCommand for Plain environment"),
        }

        // Restore original env
        if let Ok(val) = original_tmux {
            std::env::set_var("TMUX", val);
        }
        if let Ok(val) = original_zellij {
            std::env::set_var("ZELLIJ", val);
        }
    }
}
