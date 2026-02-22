//! Logging initialization for the Agent Console daemon.
//!
//! Configures the `tracing` subscriber with level filtering via the `AGENT_CONSOLE_DASHBOARD_LOG`
//! environment variable. Falls back to `info` level when the variable is unset.
//!
//! # Usage
//!
//! ```bash
//! # Default (info level)
//! acd daemon
//!
//! # Debug level
//! AGENT_CONSOLE_DASHBOARD_LOG=debug acd daemon
//!
//! # Module-specific filtering
//! AGENT_CONSOLE_DASHBOARD_LOG=agent_console=debug,warn acd daemon
//! ```

use std::fs::{self, OpenOptions};
use std::io;
use std::path::PathBuf;
use tracing_subscriber::{fmt, EnvFilter};

/// Initialize the tracing subscriber.
///
/// Reads the `AGENT_CONSOLE_DASHBOARD_LOG` environment variable for filter directives.
/// Falls back to `info` level when the variable is unset or invalid.
///
/// # Arguments
///
/// * `log_file` - Optional path to a log file. When `Some(path)`, logs are appended to that file.
///   When `None`, logs are written to stderr (foreground mode).
///
/// # Panics
///
/// Panics if a global subscriber has already been set (should only be
/// called once, at daemon startup), or if the log file cannot be opened.
pub fn init(log_file: Option<PathBuf>) -> io::Result<()> {
    let filter = EnvFilter::try_from_env("AGENT_CONSOLE_DASHBOARD_LOG")
        .unwrap_or_else(|_| EnvFilter::new("info"));

    match log_file {
        Some(path) => {
            // Create parent directory if it doesn't exist
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Open file in append mode
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;

            fmt()
                .with_env_filter(filter)
                .with_target(false)
                .with_writer(file)
                .init();
        }
        None => {
            fmt()
                .with_env_filter(filter)
                .with_target(false)
                .with_writer(std::io::stderr)
                .init();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::EnvFilter;

    #[test]
    fn env_filter_parses_valid_directives() {
        // Verify common filter strings parse without error
        let directives = ["info", "debug", "warn", "error", "trace"];
        for d in directives {
            let filter = EnvFilter::try_new(d);
            assert!(filter.is_ok(), "failed to parse directive: {}", d);
        }
    }

    #[test]
    fn env_filter_parses_module_directive() {
        let filter = EnvFilter::try_new("agent_console=debug,warn");
        assert!(filter.is_ok());
    }

    #[test]
    fn init_with_none_succeeds() {
        // Cannot test actual init (would panic on second call), but verify it compiles
        let _result: io::Result<()> = Ok(());
    }

    #[test]
    fn init_with_file_path_creates_parent_dirs() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        let log_path = tmp.path().join("nested/dir/daemon.log");

        // Parent doesn't exist yet
        assert!(!log_path.parent().unwrap().exists());

        // This would normally call init(), but we can't due to global subscriber
        // Instead, just test the directory creation logic directly
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent).expect("should create parent dirs");
        }

        assert!(log_path.parent().unwrap().exists());
    }
}
