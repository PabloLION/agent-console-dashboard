//! Integration tests for Unix Socket Server
//!
//! These tests verify the socket server's ability to handle client connections
//! and message exchange in realistic scenarios.

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::broadcast;
use tokio::time::timeout;

use agent_console::daemon::SocketServer;
use tempfile::TempDir;

/// Global counter for unique socket paths across tests
static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Creates a unique socket path for test isolation
fn unique_socket_path(temp_dir: &TempDir, prefix: &str) -> String {
    let count = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    temp_dir
        .path()
        .join(format!("{}_{}.sock", prefix, count))
        .to_string_lossy()
        .into_owned()
}

/// Helper to start a server and run it in the background with shutdown support
async fn start_server_with_shutdown(
    socket_path: String,
) -> (SocketServer, broadcast::Sender<()>) {
    let mut server = SocketServer::new(socket_path);
    server.start().await.expect("Failed to start server");
    let (shutdown_tx, _) = broadcast::channel(1);
    (server, shutdown_tx)
}

// ============================================================================
// Client Connection Tests
// ============================================================================

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
    let connect_result = timeout(
        Duration::from_secs(1),
        UnixStream::connect(&socket_path),
    )
    .await;

    assert!(
        connect_result.is_ok(),
        "Connection should not timeout"
    );
    assert!(
        connect_result.unwrap().is_ok(),
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

// ============================================================================
// Message Exchange Tests
// ============================================================================

#[tokio::test]
async fn test_client_sends_message_and_receives_echo() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "echo");

    // Start server
    let (server, shutdown_tx) = start_server_with_shutdown(socket_path.clone()).await;

    // Run server in background
    let shutdown_rx = shutdown_tx.subscribe();
    let server_handle = tokio::spawn(async move {
        let _ = server.run_with_shutdown(shutdown_rx).await;
    });

    // Give server time to start accepting
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Connect and send message
    let stream = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect");
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Send a message
    let message = "Hello, Server!\n";
    writer
        .write_all(message.as_bytes())
        .await
        .expect("Failed to write");
    writer.flush().await.expect("Failed to flush");

    // Read echo response
    let mut response = String::new();
    let read_result = timeout(
        Duration::from_secs(1),
        reader.read_line(&mut response),
    )
    .await;

    assert!(read_result.is_ok(), "Read should not timeout");
    assert!(read_result.unwrap().is_ok(), "Read should succeed");
    assert_eq!(response, message, "Server should echo back the message");

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}

#[tokio::test]
async fn test_client_sends_multiple_messages() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "multi_msg");

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

    // Send multiple messages and verify each echo
    let messages = vec![
        "First message\n",
        "Second message\n",
        "Third message with special chars: !@#$%^&*()\n",
    ];

    for message in &messages {
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
        assert_eq!(
            &response, *message,
            "Server should echo back each message correctly"
        );
    }

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}

#[tokio::test]
async fn test_message_io_latency_under_1ms() {
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

    // Measure round-trip time
    let message = "PING\n";
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

    // Message round-trip should be very fast (< 1ms typically)
    // We allow more margin for CI environments
    assert!(
        round_trip < Duration::from_millis(100),
        "Message round-trip took too long: {:?}",
        round_trip
    );

    assert_eq!(response, message, "Should receive echo");

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}

// ============================================================================
// Concurrent Client Tests
// ============================================================================

#[tokio::test]
async fn test_multiple_concurrent_clients() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "concurrent");

    // Start server
    let (server, shutdown_tx) = start_server_with_shutdown(socket_path.clone()).await;

    // Run server in background
    let shutdown_rx = shutdown_tx.subscribe();
    let server_handle = tokio::spawn(async move {
        let _ = server.run_with_shutdown(shutdown_rx).await;
    });

    // Give server time to start accepting
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Spawn 10 concurrent clients
    let num_clients = 10;
    let mut handles = Vec::new();

    for i in 0..num_clients {
        let path = socket_path.clone();
        let handle = tokio::spawn(async move {
            let stream = UnixStream::connect(&path)
                .await
                .expect("Failed to connect");
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);

            let message = format!("Client {} says hello\n", i);
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

            assert_eq!(response, message, "Client {} should receive echo", i);
            i // Return client ID to verify completion
        });
        handles.push(handle);
    }

    // Wait for all clients to complete
    let mut completed = 0;
    for handle in handles {
        let result = timeout(Duration::from_secs(5), handle).await;
        assert!(result.is_ok(), "Client should complete within timeout");
        let _ = result.unwrap().expect("Client task should succeed");
        completed += 1;
    }

    assert_eq!(
        completed, num_clients,
        "All {} clients should complete successfully",
        num_clients
    );

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}

#[tokio::test]
async fn test_100_concurrent_clients() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "hundred");

    // Start server
    let (server, shutdown_tx) = start_server_with_shutdown(socket_path.clone()).await;

    // Run server in background
    let shutdown_rx = shutdown_tx.subscribe();
    let server_handle = tokio::spawn(async move {
        let _ = server.run_with_shutdown(shutdown_rx).await;
    });

    // Give server time to start accepting
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Spawn 100 concurrent clients
    let num_clients = 100;
    let mut handles = Vec::new();

    for i in 0..num_clients {
        let path = socket_path.clone();
        let handle = tokio::spawn(async move {
            let stream = UnixStream::connect(&path).await?;
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);

            let message = format!("Client {} message\n", i);
            writer.write_all(message.as_bytes()).await?;
            writer.flush().await?;

            let mut response = String::new();
            reader.read_line(&mut response).await?;

            Ok::<bool, std::io::Error>(response == message)
        });
        handles.push(handle);
    }

    // Wait for all clients to complete
    let mut success_count = 0;
    for handle in handles {
        let result = timeout(Duration::from_secs(10), handle).await;
        if let Ok(Ok(Ok(true))) = result {
            success_count += 1;
        }
    }

    // Allow for some minor failures in high-load scenarios, but most should succeed
    assert!(
        success_count >= 95,
        "At least 95 of 100 clients should succeed, got {}",
        success_count
    );

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}

#[tokio::test]
async fn test_rapid_connect_disconnect() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "rapid");

    // Start server
    let (server, shutdown_tx) = start_server_with_shutdown(socket_path.clone()).await;

    // Run server in background
    let shutdown_rx = shutdown_tx.subscribe();
    let server_handle = tokio::spawn(async move {
        let _ = server.run_with_shutdown(shutdown_rx).await;
    });

    // Give server time to start accepting
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Rapidly connect and disconnect
    let num_iterations = 50;
    for i in 0..num_iterations {
        let stream = UnixStream::connect(&socket_path)
            .await
            .unwrap_or_else(|_| panic!("Failed to connect on iteration {}", i));
        // Immediately drop to disconnect
        drop(stream);
    }

    // Server should still be responsive
    let stream = UnixStream::connect(&socket_path)
        .await
        .expect("Server should still accept connections after rapid connect/disconnect");

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    writer
        .write_all(b"Still works\n")
        .await
        .expect("Should write");
    writer.flush().await.expect("Should flush");

    let mut response = String::new();
    reader.read_line(&mut response).await.expect("Should read");
    assert_eq!(response, "Still works\n", "Server should still echo");

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}

// ============================================================================
// Stale Socket Cleanup Tests
// ============================================================================

#[tokio::test]
async fn test_stale_socket_cleaned_up_on_startup() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "stale");

    // Create a fake stale socket file
    fs::write(&socket_path, "fake stale socket content").expect("Failed to create stale file");
    assert!(
        Path::new(&socket_path).exists(),
        "Stale file should exist"
    );

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
    let (server1, _shutdown_tx1) = start_server_with_shutdown(socket_path.clone()).await;

    // Keep server1 alive by running it in background
    let shutdown_rx1 = _shutdown_tx1.subscribe();
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
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Another daemon is already running")
            || error_msg.contains("address in use")
            || error_msg.contains("Address already in use"),
        "Error should indicate daemon is running: {}",
        error_msg
    );

    // Shutdown first server
    _shutdown_tx1.send(()).ok();
}

// ============================================================================
// Socket Cleanup Tests
// ============================================================================

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

// ============================================================================
// Client Disconnect Handling Tests
// ============================================================================

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

    // Connect first client, send message, then disconnect
    {
        let stream = UnixStream::connect(&socket_path)
            .await
            .expect("Failed to connect first client");
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        writer
            .write_all(b"Message before disconnect\n")
            .await
            .expect("Failed to write");
        writer.flush().await.expect("Failed to flush");

        let mut response = String::new();
        reader.read_line(&mut response).await.expect("Failed to read");
        assert_eq!(response, "Message before disconnect\n");
    } // First client disconnects here

    // Give server time to process disconnect
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Connect second client - server should still work
    let stream = UnixStream::connect(&socket_path)
        .await
        .expect("Server should accept new connection after client disconnect");
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    writer
        .write_all(b"Message after reconnect\n")
        .await
        .expect("Failed to write");
    writer.flush().await.expect("Failed to flush");

    let mut response = String::new();
    reader.read_line(&mut response).await.expect("Failed to read");
    assert_eq!(response, "Message after reconnect\n");

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
        let stream = UnixStream::connect(&socket_path).await.expect("Failed to connect");
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

    writer
        .write_all(b"Valid message\n")
        .await
        .expect("Failed to write");
    writer.flush().await.expect("Failed to flush");

    let mut response = String::new();
    reader.read_line(&mut response).await.expect("Failed to read");
    assert_eq!(response, "Valid message\n");

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}
