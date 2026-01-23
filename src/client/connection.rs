//! Client connection functionality with auto-start capability.
//!
//! This module provides the core connection logic for the client, including
//! automatic daemon startup when the daemon is not running.

use std::env::current_exe;
use std::error::Error;
use std::fmt;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tokio::net::UnixStream;
use tokio::time::sleep;

use crate::client::ClientResult;

/// Error types for client operations.
#[derive(Debug)]
pub enum ClientError {
    /// The daemon failed to start within the timeout period.
    ///
    /// This error occurs when the client attempts to spawn the daemon
    /// and the daemon does not become available for connection within
    /// the retry window (approximately 5 seconds).
    DaemonStartFailed,

    /// Failed to spawn the daemon process.
    ///
    /// This error occurs when the attempt to execute the daemon binary
    /// fails, typically due to the binary not being found or permission issues.
    SpawnFailed(std::io::Error),

    /// Failed to determine the current executable path.
    ///
    /// This error occurs when `std::env::current_exe()` fails, which is
    /// needed to spawn the daemon using the same binary.
    ExecutableNotFound(std::io::Error),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::DaemonStartFailed => {
                write!(f, "Daemon failed to start within timeout period")
            }
            ClientError::SpawnFailed(e) => {
                write!(f, "Failed to spawn daemon process: {}", e)
            }
            ClientError::ExecutableNotFound(e) => {
                write!(f, "Failed to find current executable: {}", e)
            }
        }
    }
}

impl Error for ClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ClientError::DaemonStartFailed => None,
            ClientError::SpawnFailed(e) => Some(e),
            ClientError::ExecutableNotFound(e) => Some(e),
        }
    }
}

/// Client for communicating with the Agent Console daemon.
///
/// The `Client` struct wraps a Unix socket connection to the daemon
/// and provides methods for sending and receiving messages.
///
/// # Example
///
/// ```no_run
/// use agent_console::client::{connect_with_auto_start, Client};
/// use std::path::Path;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// let client = connect_with_auto_start(Path::new("/tmp/agent-console.sock")).await?;
/// // Use client for communication
/// # Ok(())
/// # }
/// ```
pub struct Client {
    /// The underlying Unix socket stream.
    stream: UnixStream,
}

impl Client {
    /// Creates a new `Client` from an established `UnixStream` connection.
    ///
    /// This is typically called internally by `connect_with_auto_start()`,
    /// but can be used directly if you have an existing connection.
    ///
    /// # Arguments
    ///
    /// * `stream` - An established Unix socket connection to the daemon.
    ///
    /// # Returns
    ///
    /// A new `Client` instance wrapping the provided stream.
    pub fn new(stream: UnixStream) -> Self {
        Self { stream }
    }

    /// Returns a reference to the underlying `UnixStream`.
    ///
    /// This can be used for direct socket operations when needed.
    pub fn stream(&self) -> &UnixStream {
        &self.stream
    }

    /// Returns a mutable reference to the underlying `UnixStream`.
    ///
    /// This can be used for direct socket operations when needed.
    pub fn stream_mut(&mut self) -> &mut UnixStream {
        &mut self.stream
    }

    /// Consumes the `Client` and returns the underlying `UnixStream`.
    ///
    /// This is useful when you need to take ownership of the stream
    /// for custom protocol handling.
    pub fn into_stream(self) -> UnixStream {
        self.stream
    }
}

/// Backoff configuration for connection retries.
const INITIAL_BACKOFF_MS: u64 = 10;
const MAX_BACKOFF_MS: u64 = 500;
const MAX_RETRIES: u32 = 10;

/// Connects to the daemon, automatically starting it if not running.
///
/// This function first attempts to connect to the daemon at the specified
/// socket path. If the connection fails (indicating the daemon is not running),
/// it will spawn the daemon in the background and retry the connection with
/// exponential backoff.
///
/// # Arguments
///
/// * `socket_path` - The path to the Unix socket where the daemon listens.
///
/// # Returns
///
/// * `Ok(Client)` - A connected client on success.
/// * `Err(...)` - An error if the daemon cannot be started or connected to.
///
/// # Retry Strategy
///
/// The function uses exponential backoff with the following parameters:
/// - Initial delay: 10ms
/// - Maximum delay: 500ms
/// - Maximum retries: 10
/// - Total timeout: approximately 5 seconds
///
/// # Race Condition Handling
///
/// If multiple clients attempt to connect simultaneously with no daemon running,
/// the socket binding acts as a mutex - only one daemon instance can bind to
/// the socket, preventing duplicate daemons.
///
/// # Example
///
/// ```no_run
/// use agent_console::client::connect_with_auto_start;
/// use std::path::Path;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// let client = connect_with_auto_start(Path::new("/tmp/agent-console.sock")).await?;
/// // Use client for communication with daemon
/// # Ok(())
/// # }
/// ```
pub async fn connect_with_auto_start(socket_path: &Path) -> ClientResult<Client> {
    // Try to connect first (daemon might already be running)
    match UnixStream::connect(socket_path).await {
        Ok(stream) => {
            tracing::debug!("Connected to existing daemon at {:?}", socket_path);
            Ok(Client::new(stream))
        }
        Err(_) => {
            tracing::debug!("Daemon not running, attempting to spawn");
            // Daemon not running, try to start it
            spawn_daemon(socket_path)?;

            // Wait for daemon to be ready with exponential backoff
            let mut delay = Duration::from_millis(INITIAL_BACKOFF_MS);
            for attempt in 0..MAX_RETRIES {
                sleep(delay).await;

                match UnixStream::connect(socket_path).await {
                    Ok(stream) => {
                        tracing::info!("Connected to daemon after {} retries", attempt + 1);
                        return Ok(Client::new(stream));
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Connection attempt {} failed: {}, retrying in {:?}",
                            attempt + 1,
                            e,
                            delay
                        );
                    }
                }

                // Double the delay, but cap at MAX_BACKOFF_MS
                delay = (delay * 2).min(Duration::from_millis(MAX_BACKOFF_MS));
            }

            Err(Box::new(ClientError::DaemonStartFailed))
        }
    }
}

/// Spawns the daemon process in the background.
///
/// This function uses `std::process::Command::spawn()` to start the daemon
/// as a detached background process. The daemon is started with the
/// `--daemonize` flag to ensure it properly detaches from the terminal.
///
/// # Arguments
///
/// * `socket_path` - The path to the Unix socket the daemon should listen on.
///
/// # Returns
///
/// * `Ok(())` - If the daemon process was successfully spawned.
/// * `Err(ClientError)` - If spawning failed (e.g., binary not found).
///
/// # Note
///
/// This function only spawns the process - it does not wait for the daemon
/// to be ready. Use `connect_with_auto_start()` for the full connection flow.
fn spawn_daemon(socket_path: &Path) -> Result<(), ClientError> {
    let exe = current_exe().map_err(ClientError::ExecutableNotFound)?;

    tracing::info!("Spawning daemon from {:?}", exe);

    Command::new(&exe)
        .args(["daemon", "--daemonize", "--socket"])
        .arg(socket_path)
        .spawn()
        .map_err(ClientError::SpawnFailed)?;

    Ok(())
}

/// Calculates the backoff delay for a given attempt number.
///
/// This function implements exponential backoff with a maximum cap.
///
/// # Arguments
///
/// * `attempt` - The zero-indexed attempt number.
///
/// # Returns
///
/// The delay duration for this attempt.
#[allow(dead_code)]
fn calculate_backoff(attempt: u32) -> Duration {
    let delay_ms = INITIAL_BACKOFF_MS.saturating_mul(1 << attempt);
    Duration::from_millis(delay_ms.min(MAX_BACKOFF_MS))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_error_display() {
        let err = ClientError::DaemonStartFailed;
        assert_eq!(
            err.to_string(),
            "Daemon failed to start within timeout period"
        );
    }

    #[test]
    fn test_client_error_spawn_failed_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "binary not found");
        let err = ClientError::SpawnFailed(io_err);
        assert!(err.to_string().contains("Failed to spawn daemon process"));
    }

    #[test]
    fn test_client_error_executable_not_found_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "exe not found");
        let err = ClientError::ExecutableNotFound(io_err);
        assert!(err
            .to_string()
            .contains("Failed to find current executable"));
    }

    #[test]
    fn test_client_error_source_daemon_start_failed() {
        // DaemonStartFailed has no underlying source error
        let err = ClientError::DaemonStartFailed;
        assert!(err.source().is_none());
    }

    #[test]
    fn test_client_error_source_spawn_failed() {
        // SpawnFailed wraps an io::Error as source
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "binary not found");
        let err = ClientError::SpawnFailed(io_err);
        assert!(err.source().is_some());
    }

    #[test]
    fn test_client_error_source_executable_not_found() {
        // ExecutableNotFound wraps an io::Error as source
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "exe not found");
        let err = ClientError::ExecutableNotFound(io_err);
        assert!(err.source().is_some());
    }

    #[test]
    fn test_backoff_delays() {
        // Verify delays double: 10→20→40→80→160→320→500→500→500→500ms
        assert_eq!(calculate_backoff(0), Duration::from_millis(10));
        assert_eq!(calculate_backoff(1), Duration::from_millis(20));
        assert_eq!(calculate_backoff(2), Duration::from_millis(40));
        assert_eq!(calculate_backoff(3), Duration::from_millis(80));
        assert_eq!(calculate_backoff(4), Duration::from_millis(160));
        assert_eq!(calculate_backoff(5), Duration::from_millis(320));
        // After this point, delays are capped at MAX_BACKOFF_MS (500ms)
        assert_eq!(calculate_backoff(6), Duration::from_millis(500));
        assert_eq!(calculate_backoff(7), Duration::from_millis(500));
        assert_eq!(calculate_backoff(8), Duration::from_millis(500));
        assert_eq!(calculate_backoff(9), Duration::from_millis(500));
    }

    #[test]
    fn test_backoff_at_max_retries() {
        // Verify backoff at MAX_RETRIES boundary
        // The function is designed for use with attempts 0..MAX_RETRIES
        let delay = calculate_backoff(MAX_RETRIES - 1);
        assert_eq!(delay, Duration::from_millis(MAX_BACKOFF_MS));

        // Also verify the last configured retry
        let delay = calculate_backoff(MAX_RETRIES);
        assert_eq!(delay, Duration::from_millis(MAX_BACKOFF_MS));
    }

    #[test]
    fn test_constants() {
        // Verify retry configuration
        assert_eq!(INITIAL_BACKOFF_MS, 10);
        assert_eq!(MAX_BACKOFF_MS, 500);
        assert_eq!(MAX_RETRIES, 10);
    }

    #[test]
    fn test_client_result_type_compatibility() {
        // Verify ClientResult type is properly defined and usable
        // This test verifies type compatibility without actual socket operations
        let _result: ClientResult<()> = Ok(());
        let _err_result: ClientResult<()> = Err(Box::new(ClientError::DaemonStartFailed));
    }

    #[test]
    fn test_client_error_is_send_sync() {
        // Verify ClientError can be used in async/threaded contexts
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        // ClientError should be Send + Sync for use with async code
        assert_send::<ClientError>();
        assert_sync::<ClientError>();
    }

    #[test]
    fn test_client_error_debug() {
        // Verify Debug trait is derived
        let err = ClientError::DaemonStartFailed;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("DaemonStartFailed"));

        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let spawn_err = ClientError::SpawnFailed(io_err);
        let debug_str = format!("{:?}", spawn_err);
        assert!(debug_str.contains("SpawnFailed"));
    }
}
