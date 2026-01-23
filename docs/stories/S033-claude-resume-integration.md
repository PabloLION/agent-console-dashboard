# Story: Integrate with Claude --resume

**Story ID:** S033
**Epic:** [E008 - Session Resurrection](../epic/E008-session-resurrection.md)
**Status:** Draft
**Priority:** P2
**Estimated Points:** 5

## Description

As a user,
I want the resurrection process to properly invoke `claude --resume` in the correct context,
So that my Claude Code session continues seamlessly with full conversation history.

## Context

The RESURRECT command (S032) prepares the resurrection and validates prerequisites. This story handles the actual integration with Claude Code's `--resume` flag, ensuring the session is resumed in the correct working directory with proper terminal context. This is the most technically challenging part of session resurrection as it involves spawning processes and potentially coordinating with terminal multiplexers.

Different execution environments require different approaches: direct terminal, Zellij pane, tmux window, or displaying the command for manual execution.

## Implementation Details

### Technical Approach

1. Detect current terminal environment (Zellij, tmux, plain terminal)
2. Implement resume strategies for each environment
3. Handle working directory change before resume
4. Implement fallback strategy (print command) when environment is unsupported
5. Detect and report non-resumable sessions (context limit exceeded)
6. Update session state when resume is detected via hooks

### Files to Modify

- `src/resurrection/mod.rs` - Create resurrection module
- `src/resurrection/executor.rs` - Implement resume execution strategies
- `src/resurrection/environment.rs` - Detect terminal environment
- `src/ipc/commands/resurrect.rs` - Integrate executor into command
- `src/hooks/user_prompt_submit.rs` - Detect resumed session becoming active

### Dependencies

- [S032 - RESURRECT Command](./S032-resurrect-command.md) - Provides command infrastructure
- [S024 - User Prompt Submit Hook](./S024-user-prompt-submit-hook.md) - Detects session activity
- E010 - Zellij Integration (optional) - Enables Zellij pane creation

## Acceptance Criteria

- [ ] Given Zellij environment with integration enabled, when RESURRECT executes, then new pane opens with resumed session
- [ ] Given plain terminal environment, when RESURRECT executes with --execute, then claude --resume runs in current terminal
- [ ] Given any environment, when RESURRECT executes without --execute, then correct command is printed to stdout
- [ ] Given a session that has exceeded context limit, when resume is attempted, then appropriate error is returned
- [ ] Given successful resume, when session becomes active, then it appears in active sessions list
- [ ] Given the working directory, when resume executes, then claude starts in that directory
- [ ] Given tmux environment, when RESURRECT executes, then new tmux window/pane opens with resumed session

## Testing Requirements

- [ ] Unit test: Environment detection identifies Zellij correctly
- [ ] Unit test: Environment detection identifies tmux correctly
- [ ] Unit test: Environment detection identifies plain terminal
- [ ] Unit test: Resume command generation is correct
- [ ] Unit test: Working directory is set correctly before resume
- [ ] Integration test: Zellij pane creation with resume command
- [ ] Integration test: Direct execution in plain terminal
- [ ] Manual test: Full resurrection flow with Claude Code

## Out of Scope

- Implementing full Zellij plugin integration (E010)
- Persisting resurrection state across daemon restarts
- Automatic detection of which pane to use for resurrection
- Multi-session batch resurrection

## Notes

### Environment Detection

```rust
/// Detected terminal environment
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TerminalEnvironment {
    /// Running inside Zellij
    Zellij,
    /// Running inside tmux
    Tmux,
    /// Running inside screen
    Screen,
    /// Plain terminal, no multiplexer
    Plain,
    /// Unknown environment
    Unknown,
}

impl TerminalEnvironment {
    pub fn detect() -> Self {
        // Check for Zellij
        if std::env::var("ZELLIJ").is_ok() || std::env::var("ZELLIJ_SESSION_NAME").is_ok() {
            return Self::Zellij;
        }

        // Check for tmux
        if std::env::var("TMUX").is_ok() {
            return Self::Tmux;
        }

        // Check for screen
        if std::env::var("STY").is_ok() {
            return Self::Screen;
        }

        // Check if we're in a terminal at all
        if std::env::var("TERM").is_ok() {
            return Self::Plain;
        }

        Self::Unknown
    }
}
```

### Resume Executor

```rust
use crate::resurrection::environment::TerminalEnvironment;

pub struct ResumeExecutor {
    session_id: String,
    working_dir: PathBuf,
    environment: TerminalEnvironment,
}

impl ResumeExecutor {
    pub fn new(session_id: String, working_dir: PathBuf) -> Self {
        Self {
            session_id,
            working_dir,
            environment: TerminalEnvironment::detect(),
        }
    }

    /// Get the command to resume the session
    pub fn command(&self) -> String {
        format!("claude --resume {}", self.session_id)
    }

    /// Get the full command including directory change
    pub fn full_command(&self) -> String {
        format!(
            "cd {} && claude --resume {}",
            self.working_dir.display(),
            self.session_id
        )
    }

    /// Execute the resume based on environment
    pub fn execute(&self, options: ExecuteOptions) -> Result<ExecuteResult, ResumeError> {
        match self.environment {
            TerminalEnvironment::Zellij if options.use_multiplexer => {
                self.execute_zellij()
            }
            TerminalEnvironment::Tmux if options.use_multiplexer => {
                self.execute_tmux()
            }
            TerminalEnvironment::Plain | _ if options.execute_direct => {
                self.execute_direct()
            }
            _ => {
                // Return command for manual execution
                Ok(ExecuteResult::Command(self.full_command()))
            }
        }
    }

    /// Execute in a new Zellij pane
    fn execute_zellij(&self) -> Result<ExecuteResult, ResumeError> {
        let status = std::process::Command::new("zellij")
            .args(["run", "--cwd", self.working_dir.to_str().unwrap(), "--", "claude", "--resume", &self.session_id])
            .status()?;

        if status.success() {
            Ok(ExecuteResult::Executed)
        } else {
            Err(ResumeError::ZellijFailed(status.code()))
        }
    }

    /// Execute in a new tmux window
    fn execute_tmux(&self) -> Result<ExecuteResult, ResumeError> {
        let status = std::process::Command::new("tmux")
            .args([
                "new-window",
                "-c", self.working_dir.to_str().unwrap(),
                &format!("claude --resume {}", self.session_id)
            ])
            .status()?;

        if status.success() {
            Ok(ExecuteResult::Executed)
        } else {
            Err(ResumeError::TmuxFailed(status.code()))
        }
    }

    /// Execute directly in current terminal
    fn execute_direct(&self) -> Result<ExecuteResult, ResumeError> {
        std::env::set_current_dir(&self.working_dir)?;

        let status = std::process::Command::new("claude")
            .arg("--resume")
            .arg(&self.session_id)
            .status()?;

        if status.success() {
            Ok(ExecuteResult::Executed)
        } else {
            // Check for context limit error
            if status.code() == Some(1) {
                // May need to parse stderr for specific error
                Err(ResumeError::SessionNotResumable(
                    "Session may have exceeded context limit".to_string()
                ))
            } else {
                Err(ResumeError::ClaudeFailed(status.code()))
            }
        }
    }
}

#[derive(Debug)]
pub enum ExecuteResult {
    /// Command was executed successfully
    Executed,
    /// Command string for manual execution
    Command(String),
}

pub struct ExecuteOptions {
    /// Execute directly (vs. returning command string)
    pub execute_direct: bool,
    /// Use terminal multiplexer if available
    pub use_multiplexer: bool,
}
```

### Detecting Non-Resumable Sessions

Claude Code returns an error when attempting to resume a session that has exceeded its context limit. We need to handle this gracefully:

```rust
/// Check if a session is resumable by attempting a dry run
pub fn check_resumable(session_id: &str) -> Result<bool, ResumeError> {
    // Unfortunately, there's no way to check without attempting
    // We could parse Claude's output/error for specific messages

    // For now, assume all closed sessions are resumable
    // and handle the error at execution time
    Ok(true)
}

/// Parse Claude Code error output to detect context limit exceeded
fn parse_resume_error(stderr: &str) -> Option<String> {
    if stderr.contains("context limit") || stderr.contains("cannot resume") {
        Some("Session has exceeded context limit and cannot be resumed".to_string())
    } else {
        None
    }
}
```

### Session Re-registration

When a session is successfully resumed, it will trigger the UserPromptSubmit hook which registers it as an active session again. The hook should recognize this as a resumed session:

```rust
// In UserPromptSubmit hook handler
pub fn handle_user_prompt_submit(session_id: &str, working_dir: &Path) {
    let store = get_store();

    // Check if this was a closed session being resumed
    if store.was_closed(session_id) {
        tracing::info!("Detected resumed session: {}", session_id);
        store.mark_resumed(session_id);
    }

    // Register/update the active session
    store.update_session(session_id, SessionStatus::Working, working_dir);
}
```

### Configuration Options

```toml
[resurrection]
# Prefer using terminal multiplexer for new panes
prefer_multiplexer = true

# Default action: "execute", "print", or "copy"
# - execute: Run claude --resume directly
# - print: Print the command to stdout
# - copy: Copy the command to clipboard (if available)
default_action = "print"
```

### Challenges

1. **Terminal Context**: The daemon process may not have access to the terminal where the user wants to run the resumed session. The safest approach is often to return the command for manual execution.

2. **Clipboard Access**: Copying to clipboard requires additional dependencies and may not work in all environments.

3. **Zellij/tmux Session**: Creating new panes requires the multiplexer to be running and accessible. The daemon may be running outside the multiplexer session.

4. **Context Limit Detection**: Claude Code's error messages for non-resumable sessions need to be parsed reliably.
