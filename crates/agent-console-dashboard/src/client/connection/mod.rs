//! Client connection functionality with lazy-start capability.
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
    ///
    /// Contains the last connection error for diagnostic purposes.
    DaemonStartFailed {
        /// Number of connection attempts made.
        attempts: u32,
        /// The last error encountered during retry attempts.
        last_error: Option<std::io::Error>,
    },

    /// Connection failed with a non-recoverable error.
    ///
    /// This error occurs when the initial connection attempt fails with
    /// an error that cannot be resolved by lazy-starting the daemon
    /// (e.g., permission denied, invalid path).
    ConnectionFailed(std::io::Error),

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
            ClientError::DaemonStartFailed {
                attempts,
                last_error,
            } => {
                write!(
                    f,
                    "Daemon failed to start after {} attempts. \
                    Last error: {}. \
                    Try: 1) Check socket path permissions, 2) Check for existing daemon, \
                    3) Verify binary has execute permissions",
                    attempts,
                    last_error
                        .as_ref()
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                )
            }
            ClientError::ConnectionFailed(e) => {
                write!(
                    f,
                    "Connection to daemon failed: {}. \
                    This error cannot be resolved by lazy-starting the daemon",
                    e
                )
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
            ClientError::DaemonStartFailed { last_error, .. } => {
                last_error.as_ref().map(|e| e as &(dyn Error + 'static))
            }
            ClientError::ConnectionFailed(e) => Some(e),
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
/// ```ignore
/// use crate::client::{connect_with_lazy_start, Client};
/// use std::path::Path;
///
/// async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///     let client = connect_with_lazy_start(Path::new("/tmp/agent-console.sock")).await?;
///     // Use client for communication
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct Client {
    /// The underlying Unix socket stream.
    stream: UnixStream,
}

impl Client {
    /// Creates a new `Client` from an established `UnixStream` connection.
    ///
    /// This is typically called internally by `connect_with_lazy_start()`,
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
/// ```ignore
/// use crate::client::connect_with_lazy_start;
/// use std::path::Path;
///
/// async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///     let client = connect_with_lazy_start(Path::new("/tmp/agent-console.sock")).await?;
///     // Use client for communication with daemon
///     Ok(())
/// }
/// ```
pub async fn connect_with_lazy_start(socket_path: &Path) -> ClientResult<Client> {
    // Try to connect first (daemon might already be running)
    match UnixStream::connect(socket_path).await {
        Ok(stream) => {
            tracing::debug!("Connected to existing daemon at {:?}", socket_path);
            return Ok(Client::new(stream));
        }
        Err(e) => {
            // Only attempt lazy-start for errors indicating daemon is not running
            match e.kind() {
                std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound => {
                    tracing::info!(
                        "Daemon not running at {:?} ({}), attempting lazy-start",
                        socket_path,
                        e
                    );
                }
                _ => {
                    // Non-recoverable errors: permission denied, invalid path, etc.
                    tracing::error!(
                        "Connection to daemon at {:?} failed with non-recoverable error: {}",
                        socket_path,
                        e
                    );
                    return Err(Box::new(ClientError::ConnectionFailed(e)));
                }
            }
        }
    }

    // Daemon not running, try to start it
    spawn_daemon(socket_path)?;

    // Wait for daemon to be ready with exponential backoff
    let mut last_error: Option<std::io::Error> = None;

    for attempt in 0..MAX_RETRIES {
        let delay = calculate_backoff(attempt);
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
                last_error = Some(e);
            }
        }
    }

    Err(Box::new(ClientError::DaemonStartFailed {
        attempts: MAX_RETRIES,
        last_error,
    }))
}

/// Spawns the daemon process in the background.
///
/// This function uses `std::process::Command::spawn()` to start the daemon
/// as a detached background process. The daemon is started with the
/// `--detach` flag to ensure it properly detaches from the terminal.
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
/// to be ready. Use `connect_with_lazy_start()` for the full connection flow.
fn spawn_daemon(socket_path: &Path) -> Result<(), ClientError> {
    let exe = current_exe().map_err(ClientError::ExecutableNotFound)?;

    tracing::info!("Spawning daemon from {:?}", exe);

    let child = Command::new(&exe)
        .args(["daemon", "start", "--detach", "--socket"])
        .arg(socket_path)
        .spawn()
        .map_err(ClientError::SpawnFailed)?;

    tracing::info!("Daemon spawned successfully with PID {}", child.id());
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
fn calculate_backoff(attempt: u32) -> Duration {
    let delay_ms = INITIAL_BACKOFF_MS.saturating_mul(1 << attempt);
    Duration::from_millis(delay_ms.min(MAX_BACKOFF_MS))
}

#[cfg(test)]
mod tests;
