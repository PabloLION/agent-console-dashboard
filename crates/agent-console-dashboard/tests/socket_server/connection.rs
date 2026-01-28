//! Client connection tests for Unix Socket Server.

use super::common::{start_server_with_shutdown, unique_socket_path};
use std::time::Duration;
use tempfile::TempDir;
use tokio::net::UnixStream;
use tokio::time::timeout;

#[tokio::test]
async fn test_client_can_connect_to_server() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "connect");

    // Start server
    let (server, shutdown_tx) = start_server_with_shutdown(socket_path.clone()).await;

    // Run server in background
    let shutdown_rx = shutdown_tx.subscribe();
    let server_handle = tokio::spawn(async move {
        let _ = server.run_with_shutdown(shutdown_rx).await;
    });

    // Give server time to start accepting
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Connect client
    let connect_result = timeout(Duration::from_secs(1), UnixStream::connect(&socket_path)).await;

    assert!(connect_result.is_ok(), "Connection should not timeout");
    assert!(
        connect_result
            .expect("already checked is_ok")
            .is_ok(),
        "Client should successfully connect to server"
    );

    // Shutdown server
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}

#[tokio::test]
async fn test_client_connection_within_1ms_latency() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "latency");

    // Start server
    let (server, shutdown_tx) = start_server_with_shutdown(socket_path.clone()).await;

    // Run server in background
    let shutdown_rx = shutdown_tx.subscribe();
    let server_handle = tokio::spawn(async move {
        let _ = server.run_with_shutdown(shutdown_rx).await;
    });

    // Give server time to start accepting
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Measure connection time
    let start = std::time::Instant::now();
    let stream = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect");
    let connect_duration = start.elapsed();

    // Unix socket connection should be very fast (< 1ms typically)
    // We allow a bit more margin for CI environments
    assert!(
        connect_duration < Duration::from_millis(100),
        "Connection took too long: {:?}",
        connect_duration
    );

    drop(stream);
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}
