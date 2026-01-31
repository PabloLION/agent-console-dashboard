//! Unix Socket Server for IPC
//!
//! This module implements the Unix socket server that enables IPC (Inter-Process Communication)
//! between the Agent Console daemon and its clients (Claude Code hooks and TUI dashboards).
//!
//! The socket server provides:
//! - Local-only communication with sub-millisecond latency
//! - Zero network configuration
//! - Filesystem-based access control
//! - Support for 100+ concurrent clients
//!
//! # Example
//!
//! ```no_run
//! use agent_console::daemon::SocketServer;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     let mut server = SocketServer::new("/tmp/agent-console.sock".to_string());
//!     server.start().await?;
//!     server.run().await?;
//!     Ok(())
//! }
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::broadcast;

use crate::daemon::store::SessionStore;
use crate::{
    get_memory_usage_mb, AgentType, DaemonDump, HealthStatus, SessionCounts, SessionSnapshot,
    Status,
};

/// Unix socket server for daemon IPC.
///
/// The `SocketServer` handles:
/// - Socket creation and binding
/// - Stale socket cleanup on startup
/// - Graceful socket cleanup on shutdown (via Drop)
/// - Connection acceptance and handling
/// - Concurrent client management
pub struct SocketServer {
    /// Path to the Unix socket file
    socket_path: String,
    /// The Unix listener, set after start() is called
    listener: Option<UnixListener>,
    /// Thread-safe session store for managing agent sessions
    store: SessionStore,
    /// Timestamp when the server was created (for uptime calculation).
    start_time: Instant,
    /// Count of currently active client connections.
    active_connections: Arc<AtomicUsize>,
}

impl SocketServer {
    /// Creates a new `SocketServer` with the specified socket path.
    ///
    /// The server is not started until `start()` is called.
    ///
    /// # Arguments
    ///
    /// * `socket_path` - The filesystem path where the Unix socket will be created.
    ///   Default is typically `/tmp/agent-console.sock`.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_console::daemon::SocketServer;
    ///
    /// let server = SocketServer::new("/tmp/my-daemon.sock".to_string());
    /// ```
    pub fn new(socket_path: String) -> Self {
        tracing::debug!("Creating SocketServer with path: {}", socket_path);
        Self {
            socket_path,
            listener: None,
            store: SessionStore::new(),
            start_time: Instant::now(),
            active_connections: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Returns the configured socket path.
    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }

    /// Returns the daemon start time.
    pub fn start_time(&self) -> Instant {
        self.start_time
    }

    /// Returns the count of active connections.
    pub fn active_connection_count(&self) -> usize {
        self.active_connections.load(Ordering::Relaxed)
    }

    /// Cleans up a stale socket file from a previous daemon crash.
    ///
    /// This method checks if a socket file already exists:
    /// - If the file exists and a daemon is running (can connect), returns an error
    /// - If the file exists but is stale (cannot connect), removes it
    /// - If the file does not exist, does nothing
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` with `AddrInUse` if another daemon is already running.
    async fn cleanup_stale_socket(&self) -> std::io::Result<()> {
        let path = Path::new(&self.socket_path);

        if path.exists() {
            tracing::debug!(
                "Socket file exists at {}, checking if daemon is running",
                self.socket_path
            );

            // Try to connect to check if daemon is actually running
            match UnixStream::connect(&self.socket_path).await {
                Ok(_) => {
                    // Daemon is running, return error
                    tracing::error!("Another daemon is already running at {}", self.socket_path);
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::AddrInUse,
                        "Another daemon is already running",
                    ));
                }
                Err(_) => {
                    // Stale socket, remove it
                    tracing::info!("Removing stale socket file at {}", self.socket_path);
                    fs::remove_file(path)?;
                }
            }
        }

        Ok(())
    }

    /// Starts the server by cleaning up any stale socket and binding to the socket path.
    ///
    /// This method must be called before `run()`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Another daemon is already running (socket is in use)
    /// - Cannot remove stale socket file (permission denied)
    /// - Cannot bind to the socket path (permission denied, directory doesn't exist)
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Clean up stale socket file if it exists
        self.cleanup_stale_socket().await?;

        // Bind to socket
        tracing::info!("Binding to socket: {}", self.socket_path);
        let listener = UnixListener::bind(&self.socket_path)?;
        self.listener = Some(listener);

        tracing::info!("Socket server started at {}", self.socket_path);
        Ok(())
    }

    /// Runs the server accept loop, spawning a task for each client connection.
    ///
    /// This method runs indefinitely until an error occurs.
    /// Each client connection is handled in a separate Tokio task.
    ///
    /// # Panics
    ///
    /// Panics if called before `start()`.
    ///
    /// # Errors
    ///
    /// Returns an error if the accept loop fails (unlikely for Unix sockets).
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = self
            .listener
            .as_ref()
            .expect("Server not started - call start() first");

        tracing::info!("Socket server running, accepting connections...");

        let daemon_state = DaemonState {
            store: self.store.clone(),
            start_time: self.start_time,
            active_connections: Arc::clone(&self.active_connections),
            socket_path: self.socket_path.clone(),
        };

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    tracing::debug!("Accepted new client connection");
                    let state = daemon_state.clone();
                    tokio::spawn(async move {
                        state.active_connections.fetch_add(1, Ordering::Relaxed);
                        let result = handle_client(stream, &state).await;
                        state.active_connections.fetch_sub(1, Ordering::Relaxed);
                        if let Err(e) = result {
                            tracing::warn!("Client handler error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    // Log error but continue accepting other connections
                    tracing::error!("Accept error: {}", e);
                }
            }
        }
    }

    /// Runs the server with graceful shutdown support.
    ///
    /// This method runs the accept loop until a shutdown signal is received.
    ///
    /// # Arguments
    ///
    /// * `shutdown_rx` - A broadcast receiver that signals when to shut down.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tokio::sync::broadcast;
    /// use agent_console::daemon::SocketServer;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ///     let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    ///     let mut server = SocketServer::new("/tmp/agent-console.sock".to_string());
    ///     server.start().await?;
    ///
    ///     // In another task: shutdown_tx.send(()).unwrap();
    ///     server.run_with_shutdown(shutdown_rx).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn run_with_shutdown(
        &self,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = self
            .listener
            .as_ref()
            .expect("Server not started - call start() first");

        tracing::info!("Socket server running with shutdown support...");

        let daemon_state = DaemonState {
            store: self.store.clone(),
            start_time: self.start_time,
            active_connections: Arc::clone(&self.active_connections),
            socket_path: self.socket_path.clone(),
        };

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, _addr)) => {
                            tracing::debug!("Accepted new client connection");
                            let state = daemon_state.clone();
                            tokio::spawn(async move {
                                state.active_connections.fetch_add(1, Ordering::Relaxed);
                                let result = handle_client(stream, &state).await;
                                state.active_connections.fetch_sub(1, Ordering::Relaxed);
                                if let Err(e) = result {
                                    tracing::warn!("Client handler error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("Accept error: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("Shutdown signal received, stopping server");
                    break;
                }
            }
        }

        Ok(())
    }
}

impl Drop for SocketServer {
    /// Cleans up the socket file on drop (best-effort).
    fn drop(&mut self) {
        let path = Path::new(&self.socket_path);
        if path.exists() {
            tracing::debug!("Cleaning up socket file: {}", self.socket_path);
            if let Err(e) = fs::remove_file(path) {
                tracing::error!(
                    "Failed to remove socket file '{}': {}. \
                    Next daemon start may fail. Manually remove the file if needed.",
                    self.socket_path,
                    e
                );
            }
        }
    }
}

/// Shared daemon state passed to each client handler.
#[derive(Clone)]
struct DaemonState {
    store: SessionStore,
    start_time: Instant,
    active_connections: Arc<AtomicUsize>,
    socket_path: String,
}

/// Handles a single client connection.
///
/// This function reads commands from the client and processes them:
/// - SET <session_id> <status> [working_dir] - Create or update session
/// - RM <session_id> - Close session (mark as closed, don't remove)
/// - LIST - List all sessions
/// - GET <session_id> - Get a single session
/// - SUB - Subscribe to session updates
/// - STATUS - Return daemon health information as JSON
///
/// # Arguments
///
/// * `stream` - The Unix stream connected to the client.
/// * `state` - Shared daemon state including store, start time, and connection tracking.
///
/// # Errors
///
/// Returns an error if reading or writing fails.
async fn handle_client(
    stream: UnixStream,
    state: &DaemonState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    tracing::debug!("Client handler started");

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;

        if bytes_read == 0 {
            // Connection closed by client
            tracing::debug!("Client disconnected");
            break;
        }

        let trimmed = line.trim();
        let parts: Vec<&str> = trimmed.split_whitespace().collect();

        if parts.is_empty() {
            writer.write_all(b"ERR empty command\n").await?;
            writer.flush().await?;
            continue;
        }

        let command = parts[0].to_uppercase();
        let response = match command.as_str() {
            "SET" => handle_set_command(&parts[1..], &state.store).await,
            "RM" => handle_rm_command(&parts[1..], &state.store).await,
            "LIST" => handle_list_command(&state.store).await,
            "GET" => handle_get_command(&parts[1..], &state.store).await,
            "STATUS" => handle_status_command(state).await,
            "DUMP" => handle_dump_command(state).await,
            "SUB" => {
                // Subscribe mode: send UPDATE notifications to client
                handle_sub_command(&state.store, &mut writer).await?;
                // After SUB returns (client disconnected or error), exit the loop
                break;
            }
            _ => format!("ERR unknown command: {}\n", parts[0]),
        };

        writer.write_all(response.as_bytes()).await?;
        writer.flush().await?;
    }

    Ok(())
}

/// Handles the SET command: SET <session_id> <status> [working_dir]
///
/// Creates a new session if it doesn't exist, or updates the status if it does.
async fn handle_set_command(args: &[&str], store: &SessionStore) -> String {
    if args.len() < 2 {
        return "ERR SET requires: <session_id> <status> [working_dir]\n".to_string();
    }

    let session_id = args[0];
    let status_str = args[1];
    let working_dir = if args.len() > 2 {
        PathBuf::from(args[2])
    } else {
        PathBuf::from("unknown")
    };

    let status: Status = match status_str.parse() {
        Ok(s) => s,
        Err(_) => {
            return format!(
                "ERR invalid status: {} (expected: working, attention, question, closed)\n",
                status_str
            );
        }
    };

    // Get or create the session (lazy creation)
    let _session = store
        .get_or_create_session(
            session_id.to_string(),
            AgentType::ClaudeCode,
            working_dir,
            None, // session_id for resume capability not provided in basic command
        )
        .await;

    // Update the session status (this will notify subscribers if status changed)
    match store.update_session(session_id, status).await {
        Some(session) => {
            format!("OK {} {}\n", session.id, session.status)
        }
        None => {
            // This should never happen - log as error for investigation
            tracing::error!(
                "BUG: session '{}' not found immediately after get_or_create_session",
                session_id
            );
            format!("ERR session not found: {}\n", session_id)
        }
    }
}

/// Handles the RM command: RM <session_id>
///
/// Closes the session (marks as closed, doesn't remove from store).
async fn handle_rm_command(args: &[&str], store: &SessionStore) -> String {
    if args.is_empty() {
        return "ERR RM requires: <session_id>\n".to_string();
    }

    let session_id = args[0];

    match store.close_session(session_id).await {
        Some(session) => {
            format!("OK {} closed\n", session.id)
        }
        None => {
            format!("ERR session not found: {}\n", session_id)
        }
    }
}

/// Handles the LIST command: LIST
///
/// Returns all sessions in the format: OK\n<session_id> <status> <elapsed_seconds>\n...
async fn handle_list_command(store: &SessionStore) -> String {
    let sessions = store.list_all().await;

    if sessions.is_empty() {
        return "OK\n".to_string();
    }

    let mut response = String::from("OK\n");
    for session in sessions {
        let elapsed = session.since.elapsed().as_secs();
        response.push_str(&format!("{} {} {}\n", session.id, session.status, elapsed));
    }

    response
}

/// Handles the GET command: GET <session_id>
///
/// Returns a single session in the format: OK <session_id> <status> <elapsed_seconds> <working_dir>
async fn handle_get_command(args: &[&str], store: &SessionStore) -> String {
    if args.is_empty() {
        return "ERR GET requires: <session_id>\n".to_string();
    }

    let session_id = args[0];

    match store.get(session_id).await {
        Some(session) => {
            let elapsed = session.since.elapsed().as_secs();
            format!(
                "OK {} {} {} {}\n",
                session.id,
                session.status,
                elapsed,
                session.working_dir.display()
            )
        }
        None => {
            format!("ERR session not found: {}\n", session_id)
        }
    }
}

/// Handles the SUB command: SUB
///
/// Subscribes to session updates and sends UPDATE notifications to the client.
/// Format: UPDATE <session_id> <status> <elapsed_seconds>\n
///
/// This function runs until the client disconnects or an error occurs.
async fn handle_sub_command(
    store: &SessionStore,
    writer: &mut tokio::net::unix::OwnedWriteHalf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    writer.write_all(b"OK subscribed\n").await?;
    writer.flush().await?;

    let mut rx = store.subscribe();

    tracing::debug!("Client subscribed to session updates");

    loop {
        match rx.recv().await {
            Ok(update) => {
                let message = format!(
                    "UPDATE {} {} {}\n",
                    update.session_id, update.status, update.elapsed_seconds
                );
                if let Err(e) = writer.write_all(message.as_bytes()).await {
                    tracing::debug!("Subscriber disconnected (write failed): {}", e);
                    break;
                }
                if let Err(e) = writer.flush().await {
                    tracing::debug!("Subscriber disconnected (flush failed): {}", e);
                    break;
                }
            }
            Err(broadcast::error::RecvError::Closed) => {
                // Channel closed
                tracing::debug!("Subscriber channel closed");
                break;
            }
            Err(broadcast::error::RecvError::Lagged(count)) => {
                tracing::warn!("Subscriber lagged, missed {} messages", count);
                // Notify client of missed messages so they can refresh if needed
                let lag_message = format!("WARN lagged {}\n", count);
                if let Err(e) = writer.write_all(lag_message.as_bytes()).await {
                    tracing::debug!("Failed to notify client of lag: {}", e);
                    break;
                }
                let _ = writer.flush().await;
            }
        }
    }

    Ok(())
}

/// Handles the STATUS command: STATUS
///
/// Returns daemon health information as JSON.
/// Format: OK <json>\n
async fn handle_status_command(state: &DaemonState) -> String {
    let sessions = state.store.list_all().await;
    let active_count = sessions.iter().filter(|s| !s.closed).count();
    let closed_count = sessions.iter().filter(|s| s.closed).count();

    let health = HealthStatus {
        uptime_seconds: state.start_time.elapsed().as_secs(),
        sessions: SessionCounts {
            active: active_count,
            closed: closed_count,
        },
        connections: state.active_connections.load(Ordering::Relaxed),
        memory_mb: get_memory_usage_mb(),
        socket_path: state.socket_path.clone(),
    };

    let json = serde_json::to_string(&health).expect("failed to serialize HealthStatus");
    format!("OK {}\n", json)
}

/// Handles the DUMP command: DUMP
///
/// Returns a full daemon state snapshot as JSON.
/// Format: OK <json>\n
async fn handle_dump_command(state: &DaemonState) -> String {
    let sessions = state.store.list_all().await;
    let active_count = sessions.iter().filter(|s| !s.closed).count();
    let closed_count = sessions.iter().filter(|s| s.closed).count();

    let snapshots: Vec<SessionSnapshot> = sessions
        .iter()
        .map(|s| SessionSnapshot {
            id: s.id.clone(),
            status: s.status.to_string(),
            working_dir: s.working_dir.display().to_string(),
            elapsed_seconds: s.since.elapsed().as_secs(),
            closed: s.closed,
        })
        .collect();

    let dump = DaemonDump {
        uptime_seconds: state.start_time.elapsed().as_secs(),
        socket_path: state.socket_path.clone(),
        sessions: snapshots,
        session_counts: SessionCounts {
            active: active_count,
            closed: closed_count,
        },
        connections: state.active_connections.load(Ordering::Relaxed),
    };

    let json = serde_json::to_string(&dump).expect("failed to serialize DaemonDump");
    format!("OK {}\n", json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_server_new() {
        let server = SocketServer::new("/tmp/test-socket.sock".to_string());
        assert_eq!(server.socket_path(), "/tmp/test-socket.sock");
        assert!(server.listener.is_none());
    }

    #[test]
    fn test_socket_path_getter() {
        let path = "/tmp/custom-path.sock".to_string();
        let server = SocketServer::new(path.clone());
        assert_eq!(server.socket_path(), path);
    }
}
