//! Integration tests for Unix Socket Server
//!
//! These tests verify the socket server's ability to handle client connections
//! and message exchange in realistic scenarios.
//!
//! # Status: SKIPPED
//!
//! These tests are currently disabled because `SocketServer` is not yet
//! implemented. When implementing the socket server (likely in a future story),
//! remove the `#[cfg(feature = "socket_server")]` attributes to enable tests.
//!
//! TODO: Enable when `daemon::SocketServer` is implemented and exported.
//!
//! # Test Categories
//!
//! - `connection`: Client connection and latency tests
//! - `message`: Message exchange and echo tests
//! - `concurrent`: Concurrent client handling
//! - `cleanup`: Socket cleanup and stale socket handling
//! - `disconnect`: Client disconnect handling
//! - `lifecycle`: Session lifecycle integration
//! - `notification`: Subscriber notification tests

#[cfg(feature = "socket_server")]
mod cleanup;
#[cfg(feature = "socket_server")]
mod concurrent;
#[cfg(feature = "socket_server")]
mod connection;
#[cfg(feature = "socket_server")]
mod disconnect;
#[cfg(feature = "socket_server")]
mod lifecycle;
#[cfg(feature = "socket_server")]
mod message;
#[cfg(feature = "socket_server")]
mod notification;

#[cfg(feature = "socket_server")]
pub(crate) mod common {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use agent_console::daemon::SocketServer;
    use tempfile::TempDir;
    use tokio::sync::broadcast;

    /// Global counter for unique socket paths across tests
    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// Creates a unique socket path for test isolation
    pub fn unique_socket_path(temp_dir: &TempDir, prefix: &str) -> String {
        let count = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        temp_dir
            .path()
            .join(format!("{}_{}.sock", prefix, count))
            .to_string_lossy()
            .into_owned()
    }

    /// Helper to start a server and run it in the background with shutdown support
    pub async fn start_server_with_shutdown(
        socket_path: String,
    ) -> (SocketServer, broadcast::Sender<()>) {
        let mut server = SocketServer::new(socket_path);
        server.start().await.expect("Failed to start server");
        let (shutdown_tx, _) = broadcast::channel(1);
        (server, shutdown_tx)
    }
}
