//! Client disconnect handling tests for Unix Socket Server.

use super::common::{start_server_with_shutdown, unique_socket_path};
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::timeout;

#[tokio::test]
async fn test_server_handles_client_disconnect_gracefully() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "disconnect");

    // Start server
    let (server, shutdown_tx) = start_server_with_shutdown(socket_path.clone()).await;

    // Run server in background
    let shutdown_rx = shutdown_tx.subscribe();
    let server_handle = tokio::spawn(async move {
        let _ = server.run_with_shutdown(shutdown_rx).await;
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Connect first client, send command, then disconnect
    {
        let stream = UnixStream::connect(&socket_path)
            .await
            .expect("Failed to connect first client");
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Use a valid SET command before disconnect
        writer
            .write_all(b"SET disconnect-session working /tmp/before\n")
            .await
            .expect("Failed to write");
        writer.flush().await.expect("Failed to flush");

        let mut response = String::new();
        reader
            .read_line(&mut response)
            .await
            .expect("Failed to read");
        assert!(
            response.starts_with("OK disconnect-session working"),
            "Got: {}",
            response
        );
    } // First client disconnects here

    // Give server time to process disconnect
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Connect second client - server should still work
    let stream = UnixStream::connect(&socket_path)
        .await
        .expect("Server should accept new connection after client disconnect");
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Use a valid GET command to verify server still works
    writer
        .write_all(b"GET disconnect-session\n")
        .await
        .expect("Failed to write");
    writer.flush().await.expect("Failed to flush");

    let mut response = String::new();
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read");
    assert!(
        response.starts_with("OK disconnect-session working"),
        "Server should respond to GET, got: {}",
        response
    );

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}

#[tokio::test]
async fn test_server_continues_after_client_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "client_error");

    // Start server
    let (server, shutdown_tx) = start_server_with_shutdown(socket_path.clone()).await;

    // Run server in background
    let shutdown_rx = shutdown_tx.subscribe();
    let server_handle = tokio::spawn(async move {
        let _ = server.run_with_shutdown(shutdown_rx).await;
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Connect and abruptly drop without proper protocol
    for _ in 0..5 {
        let stream = UnixStream::connect(&socket_path)
            .await
            .expect("Failed to connect");
        // Write partial data and drop
        let (_, mut writer) = stream.into_split();
        let _ = writer.write_all(b"partial data no newline").await;
        // Don't flush, just drop
    }

    // Give server time to handle errors
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Server should still accept connections
    let stream = UnixStream::connect(&socket_path)
        .await
        .expect("Server should still work after client errors");
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Use a valid LIST command to verify server still responds correctly
    writer.write_all(b"LIST\n").await.expect("Failed to write");
    writer.flush().await.expect("Failed to flush");

    let mut response = String::new();
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read");
    assert!(
        response.starts_with("OK"),
        "Server should respond with OK, got: {}",
        response
    );

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}
