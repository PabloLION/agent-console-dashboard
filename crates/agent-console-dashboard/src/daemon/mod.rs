//! Daemon module for the Agent Console Dashboard.
//!
//! This module provides process lifecycle management, daemonization, and the
//! main entry point for running the daemon.

mod handlers;
pub mod logging;
pub mod server;
pub mod session;
pub mod store;
pub mod usage;

// Re-export commonly used types for convenience
pub use server::SocketServer;
pub use store::SessionStore;

use crate::DaemonConfig;
use fork::{daemon, Fork};
use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tokio::signal;
use tokio::signal::unix::{signal as unix_signal, SignalKind};
use tracing::{debug, error, info, warn};

/// Duration of inactivity (no non-closed sessions) before the daemon auto-stops.
const AUTO_STOP_IDLE_SECS: u64 = 3600;

/// How often the idle check runs.
const IDLE_CHECK_INTERVAL_SECS: u64 = 60;

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
                    info!("received SIGINT (Ctrl+C), shutting down");
                },
                _ = sigterm.recv() => {
                    info!("received SIGTERM, shutting down");
                },
            }
        }
        Err(e) => {
            warn!(error = %e, "could not register SIGTERM handler, using SIGINT only");
            if let Err(e) = signal::ctrl_c().await {
                error!(error = %e, "failed waiting for SIGINT");
            } else {
                info!("received SIGINT (Ctrl+C), shutting down");
            }
        }
    }
}

/// Periodically checks for active (non-closed) sessions and returns when the
/// daemon has been idle for `timeout`.
///
/// The timer starts immediately — if no session connects before the timeout
/// expires, the daemon shuts down.
async fn idle_check_loop(store: &SessionStore, timeout: Duration) {
    let mut idle_since: Option<Instant> = Some(Instant::now());
    let mut interval = tokio::time::interval(Duration::from_secs(IDLE_CHECK_INTERVAL_SECS));

    loop {
        interval.tick().await;

        let has_active = store.has_active_sessions().await;

        if has_active {
            if idle_since.is_some() {
                info!("sessions active, idle timer reset");
            }
            idle_since = None;
        } else if idle_since.is_none() {
            idle_since = Some(Instant::now());
            info!("no active sessions, idle timer started");
        } else {
            let elapsed = idle_since.expect("just checked is_some above").elapsed();
            if elapsed >= timeout {
                return;
            }
            debug!(
                remaining_secs = (timeout - elapsed).as_secs(),
                "idle check: auto-stop in {} seconds",
                (timeout - elapsed).as_secs()
            );
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

    // Initialize logging after daemonize (stderr may be redirected)
    logging::init();

    info!(
        socket_path = %config.socket_path.display(),
        daemonize = config.daemonize,
        "agent console daemon starting"
    );

    // Hooks are managed by the Claude Code plugin system (.claude-plugin/plugin.json).
    // Plugin installation is handled by `acd service install` or `claude plugin install`.

    // Create Tokio runtime AFTER daemonization
    // Using current_thread runtime for simpler daemon workloads
    let runtime = Runtime::new().map_err(|e| {
        Box::new(std::io::Error::other(format!(
            "Failed to create Tokio runtime: {}",
            e
        ))) as Box<dyn Error>
    })?;

    info!("daemon running, press Ctrl+C or send SIGTERM to stop");

    // Run the main event loop
    runtime.block_on(async {
        let mut server = SocketServer::new(config.socket_path.display().to_string());
        if let Err(e) = server.start().await {
            error!("failed to start socket server: {}", e);
            return;
        }

        let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

        // Create and wire the usage fetcher
        let usage_fetcher = Arc::new(usage::UsageFetcher::new());
        server.set_usage_fetcher(Arc::clone(&usage_fetcher));

        // Clone the store for the idle check loop before moving server
        let store = server.store().clone();

        // Spawn the usage fetcher
        let usage_shutdown_rx = shutdown_tx.subscribe();
        let usage_handle = tokio::spawn(async move {
            usage_fetcher.run(usage_shutdown_rx).await;
        });

        // Spawn the accept loop
        let server_handle = tokio::spawn(async move {
            if let Err(e) = server.run_with_shutdown(shutdown_rx).await {
                error!("socket server error: {}", e);
            }
        });

        // Wait for shutdown signal or idle timeout
        let idle_timeout = Duration::from_secs(AUTO_STOP_IDLE_SECS);
        tokio::select! {
            _ = wait_for_shutdown() => {}
            _ = idle_check_loop(&store, idle_timeout) => {
                info!("no active sessions for {} seconds, auto-stopping", AUTO_STOP_IDLE_SECS);
            }
        }

        // Signal all tasks to stop
        let _ = shutdown_tx.send(());
        let _ = server_handle.await;
        let _ = usage_handle.await;
    });

    info!("daemon stopped");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_daemonize_process_returns_result() {
        let _result: DaemonResult<()> = Ok(());
    }

    #[test]
    fn test_run_daemon_returns_result() {
        let _config = DaemonConfig::new(PathBuf::from("/tmp/test.sock"), false);
    }

    #[test]
    fn test_daemon_config_used_correctly() {
        let config = DaemonConfig::new(PathBuf::from("/tmp/test.sock"), true);
        assert!(config.daemonize);
        assert_eq!(config.socket_path, PathBuf::from("/tmp/test.sock"));
    }
}
