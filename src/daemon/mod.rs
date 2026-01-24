//! Daemon module for the Agent Console Dashboard.
//!
//! This module provides process lifecycle management, daemonization, and the
//! main entry point for running the daemon.

pub mod server;
pub mod store;

// Re-export commonly used types for convenience
pub use server::SocketServer;
pub use store::SessionStore;

use crate::DaemonConfig;
use fork::{daemon, Fork};
use std::error::Error;
use tokio::runtime::Runtime;
use tokio::signal;
use tokio::signal::unix::{signal as unix_signal, SignalKind};

/// Result type alias for daemon operations.
pub type DaemonResult<T> = Result<T, Box<dyn Error>>;

/// Wait for a shutdown signal (SIGINT or SIGTERM).
///
/// This async function blocks until either Ctrl+C (SIGINT) or SIGTERM
/// is received, enabling graceful shutdown of the daemon.
///
/// If SIGTERM handler registration fails, falls back to SIGINT only
/// with a warning message.
async fn wait_for_shutdown() {
    match unix_signal(SignalKind::terminate()) {
        Ok(mut sigterm) => {
            tokio::select! {
                _ = signal::ctrl_c() => {
                    eprintln!("Received SIGINT (Ctrl+C), shutting down...");
                },
                _ = sigterm.recv() => {
                    eprintln!("Received SIGTERM, shutting down...");
                },
            }
        }
        Err(e) => {
            eprintln!(
                "Warning: Could not register SIGTERM handler: {}. Using SIGINT only.",
                e
            );
            if let Err(e) = signal::ctrl_c().await {
                eprintln!("Error waiting for SIGINT: {}", e);
            } else {
                eprintln!("Received SIGINT (Ctrl+C), shutting down...");
            }
        }
    }
}

/// Daemonize the current process.
///
/// This function forks the process and detaches it from the terminal.
/// The parent process exits immediately with code 0, and the child
/// continues execution as a background daemon.
///
/// # Arguments
///
/// * `nochdir` - If false, changes the working directory to `/`.
///   If true, keeps the current working directory.
/// * `noclose` - If false, redirects stdin/stdout/stderr to /dev/null.
///   If true, keeps the standard file descriptors.
///
/// # Returns
///
/// * `Ok(())` - On success (in the child process)
/// * `Err(...)` - If the fork operation fails
///
/// # Note
///
/// This function MUST be called BEFORE starting the Tokio runtime,
/// as forking after Tokio initialization corrupts global state for
/// signal handling.
pub fn daemonize_process(nochdir: bool, noclose: bool) -> DaemonResult<()> {
    match daemon(nochdir, noclose) {
        Ok(Fork::Child) => {
            // Daemon child process continues here
            Ok(())
        }
        Ok(Fork::Parent(_)) => {
            // Parent exits immediately
            std::process::exit(0);
        }
        Err(e) => Err(Box::new(std::io::Error::other(format!(
            "Failed to daemonize: {}",
            e
        )))),
    }
}

/// Run the daemon with the given configuration.
///
/// This is the main entry point for the daemon. It performs daemonization
/// if requested, then starts the Tokio runtime and runs the main event loop.
///
/// # Arguments
///
/// * `config` - The daemon configuration containing socket path and daemonize flag.
///
/// # Returns
///
/// * `Ok(())` - On successful shutdown
/// * `Err(...)` - If the daemon fails to start or encounters an error
///
/// # Example
///
/// ```no_run
/// use agent_console::{DaemonConfig, daemon::run_daemon};
/// use std::path::PathBuf;
///
/// let config = DaemonConfig::new(
///     PathBuf::from("/tmp/agent-console.sock"),
///     false, // foreground mode
/// );
/// run_daemon(config).expect("Failed to run daemon");
/// ```
pub fn run_daemon(config: DaemonConfig) -> DaemonResult<()> {
    // CRITICAL: Daemonize BEFORE starting Tokio runtime
    // Forking after Tokio initialization corrupts global state for signal handling
    if config.daemonize {
        // Production mode: change to /, redirect stdio to /dev/null
        daemonize_process(false, false)?;
    }

    // Log startup information
    // Note: In daemonized mode with noclose=false, these won't be visible
    // but they're useful for foreground mode debugging
    eprintln!("Agent Console daemon starting...");
    eprintln!("  Socket path: {}", config.socket_path.display());
    eprintln!("  Daemonize: {}", config.daemonize);

    // Create Tokio runtime AFTER daemonization
    // Using current_thread runtime for simpler daemon workloads
    let runtime = Runtime::new().map_err(|e| {
        Box::new(std::io::Error::other(format!(
            "Failed to create Tokio runtime: {}",
            e
        ))) as Box<dyn Error>
    })?;

    eprintln!("Daemon running. Press Ctrl+C or send SIGTERM to stop.");

    // Run the main event loop
    runtime.block_on(async {
        // Wait for shutdown signal
        wait_for_shutdown().await;
    });

    eprintln!("Daemon stopped.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_daemonize_process_returns_result() {
        // We can't actually test fork in a unit test, but we can verify
        // the function signature and that DaemonResult is properly defined
        let _result: DaemonResult<()> = Ok(());
    }

    #[test]
    fn test_run_daemon_returns_result() {
        // We can't easily test run_daemon as it may daemonize,
        // but we can verify the function accepts DaemonConfig
        // Note: This test only verifies type compatibility
        let _config = DaemonConfig::new(PathBuf::from("/tmp/test.sock"), false);
    }

    #[test]
    fn test_daemon_config_used_correctly() {
        // Verify DaemonConfig fields are accessible
        let config = DaemonConfig::new(PathBuf::from("/tmp/test.sock"), true);
        assert!(config.daemonize);
        assert_eq!(config.socket_path, PathBuf::from("/tmp/test.sock"));
    }
}
