//! Stale socket cleanup and socket file cleanup tests for Unix Socket Server.

use super::common::{start_server_with_shutdown, unique_socket_path};
use agent_console::daemon::SocketServer;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tempfile::TempDir;
use tokio::net::UnixStream;
use tokio::time::timeout;

// =============================================================================
// Stale Socket Cleanup Tests
// =============================================================================

#[tokio::test]
async fn test_stale_socket_cleaned_up_on_startup() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "stale");

    // Create a fake stale socket file
    fs::write(&socket_path, "fake stale socket content").expect("Failed to create stale file");
    assert!(Path::new(&socket_path).exists(), "Stale file should exist");

    // Start server - should clean up stale file
    let mut server = SocketServer::new(socket_path.clone());
    server
        .start()
        .await
        .expect("Server should start after cleaning stale socket");

    // Socket file should exist (now as a real socket)
    assert!(
        Path::new(&socket_path).exists(),
        "Socket should exist after start"
    );

    // Verify it's a real socket by connecting
    let connect_result = UnixStream::connect(&socket_path).await;
    assert!(
        connect_result.is_ok(),
        "Should be able to connect to the new socket"
    );
}

#[tokio::test]
async fn test_server_detects_running_daemon() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "running");

    // Start first server
    let (server1, shutdown_tx1) = start_server_with_shutdown(socket_path.clone()).await;

    // Keep server1 alive by running it in background
    let shutdown_rx1 = shutdown_tx1.subscribe();
    let _server_handle = tokio::spawn(async move {
        let _ = server1.run_with_shutdown(shutdown_rx1).await;
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Try to start second server - should fail
    let mut server2 = SocketServer::new(socket_path.clone());
    let result = server2.start().await;

    assert!(
        result.is_err(),
        "Second server should fail to start when daemon is running"
    );

    // Verify error message
    let error_msg = result
        .expect_err("already checked is_err")
        .to_string();
    assert!(
        error_msg.contains("Another daemon is already running")
            || error_msg.contains("address in use")
            || error_msg.contains("Address already in use"),
        "Error should indicate daemon is running: {}",
        error_msg
    );

    // Shutdown first server
    shutdown_tx1.send(()).ok();
}

// =============================================================================
// Socket Cleanup Tests
// =============================================================================

#[tokio::test]
async fn test_socket_file_removed_after_server_drops() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "cleanup");

    {
        let mut server = SocketServer::new(socket_path.clone());
        server.start().await.expect("Failed to start server");

        // Socket should exist
        assert!(
            Path::new(&socket_path).exists(),
            "Socket should exist while server is alive"
        );
    } // Server dropped here

    // Socket should be removed
    assert!(
        !Path::new(&socket_path).exists(),
        "Socket file should be removed after server drops"
    );
}

#[tokio::test]
async fn test_socket_cleaned_up_after_shutdown_signal() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "shutdown_cleanup");
    let socket_path_clone = socket_path.clone();

    // Start server
    let (server, shutdown_tx) = start_server_with_shutdown(socket_path.clone()).await;

    // Socket should exist
    assert!(
        Path::new(&socket_path).exists(),
        "Socket should exist after start"
    );

    // Run server in background
    let shutdown_rx = shutdown_tx.subscribe();
    let server_handle = tokio::spawn(async move {
        let _ = server.run_with_shutdown(shutdown_rx).await;
        // Server is dropped here when task completes
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Send shutdown signal
    shutdown_tx.send(()).expect("Failed to send shutdown");

    // Wait for server to stop
    let _ = timeout(Duration::from_secs(1), server_handle).await;

    // Give Drop time to run
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Socket should be cleaned up after shutdown
    assert!(
        !Path::new(&socket_path_clone).exists(),
        "Socket should be removed after shutdown"
    );
}

#[tokio::test]
async fn test_new_server_can_start_after_previous_shutdown() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "restart");

    // Start and stop first server
    {
        let (server, shutdown_tx) = start_server_with_shutdown(socket_path.clone()).await;
        let shutdown_rx = shutdown_tx.subscribe();
        let server_handle = tokio::spawn(async move {
            let _ = server.run_with_shutdown(shutdown_rx).await;
        });

        tokio::time::sleep(Duration::from_millis(10)).await;
        shutdown_tx.send(()).ok();
        let _ = timeout(Duration::from_secs(1), server_handle).await;
    }

    // Socket should be cleaned up
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Start second server at same path
    let mut server2 = SocketServer::new(socket_path.clone());
    let result = server2.start().await;

    assert!(
        result.is_ok(),
        "New server should start successfully at same path: {:?}",
        result.err()
    );

    // Verify second server works
    let connect_result = UnixStream::connect(&socket_path).await;
    assert!(
        connect_result.is_ok(),
        "Should be able to connect to restarted server"
    );
}
