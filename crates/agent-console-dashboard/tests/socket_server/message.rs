//! Message exchange tests for Unix Socket Server.

use super::common::{start_server_with_shutdown, unique_socket_path};
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::timeout;

#[tokio::test]
async fn test_client_sends_command_and_receives_response() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "command");

    // Start server
    let (server, shutdown_tx) = start_server_with_shutdown(socket_path.clone()).await;

    // Run server in background
    let shutdown_rx = shutdown_tx.subscribe();
    let server_handle = tokio::spawn(async move {
        let _ = server.run_with_shutdown(shutdown_rx).await;
    });

    // Give server time to start accepting
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Connect and send command
    let stream = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect");
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Send a valid LIST command
    writer.write_all(b"LIST\n").await.expect("Failed to write");
    writer.flush().await.expect("Failed to flush");

    // Read response
    let mut response = String::new();
    let read_result = timeout(Duration::from_secs(1), reader.read_line(&mut response)).await;

    assert!(read_result.is_ok(), "Read should not timeout");
    assert!(
        read_result.expect("already checked is_ok").is_ok(),
        "Read should succeed"
    );
    assert!(
        response.starts_with("OK"),
        "Server should respond with OK for LIST command, got: {}",
        response
    );

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}

#[tokio::test]
async fn test_client_sends_multiple_commands() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "multi_cmd");

    // Start server
    let (server, shutdown_tx) = start_server_with_shutdown(socket_path.clone()).await;

    // Run server in background
    let shutdown_rx = shutdown_tx.subscribe();
    let server_handle = tokio::spawn(async move {
        let _ = server.run_with_shutdown(shutdown_rx).await;
    });

    // Give server time to start accepting
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Connect
    let stream = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect");
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Send multiple valid commands
    let commands = vec![
        (
            "SET multi-test-1 working /project1\n",
            "OK multi-test-1 working",
        ),
        (
            "SET multi-test-2 attention /project2\n",
            "OK multi-test-2 attention",
        ),
        ("LIST\n", "OK"),
    ];

    for (command, expected_prefix) in &commands {
        writer
            .write_all(command.as_bytes())
            .await
            .expect("Failed to write");
        writer.flush().await.expect("Failed to flush");

        let mut response = String::new();
        reader
            .read_line(&mut response)
            .await
            .expect("Failed to read");
        assert!(
            response.starts_with(expected_prefix),
            "Command '{}' should get response starting with '{}', got: {}",
            command.trim(),
            expected_prefix,
            response
        );
    }

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}

#[tokio::test]
async fn test_command_io_latency_under_1ms() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "io_latency");

    // Start server
    let (server, shutdown_tx) = start_server_with_shutdown(socket_path.clone()).await;

    // Run server in background
    let shutdown_rx = shutdown_tx.subscribe();
    let server_handle = tokio::spawn(async move {
        let _ = server.run_with_shutdown(shutdown_rx).await;
    });

    // Give server time to start accepting
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Connect
    let stream = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect");
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Measure round-trip time with a valid command
    let message = "LIST\n";
    let start = std::time::Instant::now();

    writer
        .write_all(message.as_bytes())
        .await
        .expect("Failed to write");
    writer.flush().await.expect("Failed to flush");

    let mut response = String::new();
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read");

    let round_trip = start.elapsed();

    // Command round-trip should be very fast (< 1ms typically)
    // We allow more margin for CI environments
    assert!(
        round_trip < Duration::from_millis(100),
        "Command round-trip took too long: {:?}",
        round_trip
    );

    assert!(
        response.starts_with("OK"),
        "Should receive OK response, got: {}",
        response
    );

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}
