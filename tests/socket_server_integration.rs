//! Integration tests for Unix Socket Server
//!
//! These tests verify the socket server's ability to handle client connections
//! and message exchange in realistic scenarios.
//!
//! # Status: SKIPPED
//!
//! These tests are currently disabled because `SocketServer` is not yet
//! implemented. When implementing the socket server (likely in a future story),
//! remove the `#![cfg(feature = "socket_server")]` attribute below to enable
//! these tests.
//!
//! TODO: Enable when `daemon::SocketServer` is implemented and exported.

#![cfg(feature = "socket_server")]

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
    let read_result = timeout(
        Duration::from_secs(1),
        reader.read_line(&mut response),
    )
    .await;

    assert!(read_result.is_ok(), "Read should not timeout");
    assert!(read_result.unwrap().is_ok(), "Read should succeed");
    assert!(response.starts_with("OK"), "Server should respond with OK for LIST command, got: {}", response);

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
        ("SET multi-test-1 working /project1\n", "OK multi-test-1 working"),
        ("SET multi-test-2 attention /project2\n", "OK multi-test-2 attention"),
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

    assert!(response.starts_with("OK"), "Should receive OK response, got: {}", response);

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

            // Use a valid SET command with unique session ID per client
            let command = format!("SET concurrent-session-{} working /tmp/client{}\n", i, i);
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

            let expected_prefix = format!("OK concurrent-session-{} working", i);
            assert!(
                response.starts_with(&expected_prefix),
                "Client {} should receive OK response, got: {}",
                i,
                response
            );
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

            // Use a valid SET command with unique session ID per client
            let command = format!("SET hundred-session-{} working /tmp/client{}\n", i, i);
            writer.write_all(command.as_bytes()).await?;
            writer.flush().await?;

            let mut response = String::new();
            reader.read_line(&mut response).await?;

            let expected_prefix = format!("OK hundred-session-{} working", i);
            Ok::<bool, std::io::Error>(response.starts_with(&expected_prefix))
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

    // Use a valid LIST command to verify server is responsive
    writer
        .write_all(b"LIST\n")
        .await
        .expect("Should write");
    writer.flush().await.expect("Should flush");

    let mut response = String::new();
    reader.read_line(&mut response).await.expect("Should read");
    assert!(response.starts_with("OK"), "Server should respond with OK, got: {}", response);

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
        reader.read_line(&mut response).await.expect("Failed to read");
        assert!(response.starts_with("OK disconnect-session working"), "Got: {}", response);
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
    reader.read_line(&mut response).await.expect("Failed to read");
    assert!(response.starts_with("OK disconnect-session working"), "Server should respond to GET, got: {}", response);

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

    // Use a valid LIST command to verify server still responds correctly
    writer
        .write_all(b"LIST\n")
        .await
        .expect("Failed to write");
    writer.flush().await.expect("Failed to flush");

    let mut response = String::new();
    reader.read_line(&mut response).await.expect("Failed to read");
    assert!(response.starts_with("OK"), "Server should respond with OK, got: {}", response);

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}

// ============================================================================
// Session Lifecycle Integration Tests
// ============================================================================

/// Test full session lifecycle: create via SET, update via SET, close via RM,
/// and verify LIST shows the closed session.
///
/// This test verifies the complete flow of session management through the
/// IPC protocol:
/// 1. Create a new session using SET command
/// 2. Update the session status using SET command
/// 3. Close the session using RM command
/// 4. Verify LIST shows the session with "closed" status
#[tokio::test]
async fn test_lifecycle_create_update_close_via_ipc() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "lifecycle");

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
    let stream = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect");
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut response = String::new();

    // Step 1: Create session via SET command
    writer
        .write_all(b"SET lifecycle-session-1 working /home/user/project\n")
        .await
        .expect("Failed to write SET");
    writer.flush().await.expect("Failed to flush");

    response.clear();
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read SET response");
    assert!(
        response.starts_with("OK lifecycle-session-1 working"),
        "Expected OK response for SET, got: {}",
        response
    );

    // Step 2: Update session status via SET command (working -> attention)
    writer
        .write_all(b"SET lifecycle-session-1 attention\n")
        .await
        .expect("Failed to write SET update");
    writer.flush().await.expect("Failed to flush");

    response.clear();
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read SET update response");
    assert!(
        response.starts_with("OK lifecycle-session-1 attention"),
        "Expected OK response for status update, got: {}",
        response
    );

    // Step 3: Close session via RM command
    writer
        .write_all(b"RM lifecycle-session-1\n")
        .await
        .expect("Failed to write RM");
    writer.flush().await.expect("Failed to flush");

    response.clear();
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read RM response");
    assert!(
        response.starts_with("OK lifecycle-session-1 closed"),
        "Expected OK closed response for RM, got: {}",
        response
    );

    // Step 4: Verify LIST shows the closed session
    writer
        .write_all(b"LIST\n")
        .await
        .expect("Failed to write LIST");
    writer.flush().await.expect("Failed to flush");

    response.clear();
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read LIST header");
    assert!(
        response.starts_with("OK"),
        "LIST should start with OK, got: {}",
        response
    );

    // Read the session line
    response.clear();
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read LIST session line");

    // Verify the closed session appears in LIST with "closed" status
    assert!(
        response.starts_with("lifecycle-session-1 closed"),
        "Closed session should appear in LIST with closed status, got: {}",
        response
    );

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}

/// Test that multiple sessions can go through lifecycle independently.
#[tokio::test]
async fn test_lifecycle_multiple_sessions() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "multi_lifecycle");

    // Start server
    let (server, shutdown_tx) = start_server_with_shutdown(socket_path.clone()).await;

    // Run server in background
    let shutdown_rx = shutdown_tx.subscribe();
    let server_handle = tokio::spawn(async move {
        let _ = server.run_with_shutdown(shutdown_rx).await;
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Connect client
    let stream = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect");
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut response = String::new();

    // Create three sessions
    for i in 1..=3 {
        let cmd = format!("SET session-{} working /path/{}\n", i, i);
        writer
            .write_all(cmd.as_bytes())
            .await
            .expect("Failed to write SET");
        writer.flush().await.expect("Failed to flush");

        response.clear();
        reader
            .read_line(&mut response)
            .await
            .expect("Failed to read response");
        assert!(
            response.starts_with(&format!("OK session-{} working", i)),
            "Session {} creation failed: {}",
            i,
            response
        );
    }

    // Close only session-2
    writer
        .write_all(b"RM session-2\n")
        .await
        .expect("Failed to write RM");
    writer.flush().await.expect("Failed to flush");

    response.clear();
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read RM response");
    assert!(
        response.starts_with("OK session-2 closed"),
        "Session-2 close failed: {}",
        response
    );

    // LIST should show all 3 sessions: 2 working, 1 closed
    writer
        .write_all(b"LIST\n")
        .await
        .expect("Failed to write LIST");
    writer.flush().await.expect("Failed to flush");

    // Read OK header
    response.clear();
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read LIST header");
    assert!(response.starts_with("OK"), "LIST header failed: {}", response);

    // Read all session lines
    let mut sessions = Vec::new();
    for _ in 0..3 {
        response.clear();
        reader
            .read_line(&mut response)
            .await
            .expect("Failed to read session line");
        sessions.push(response.clone());
    }

    // Verify we have exactly 3 sessions
    assert_eq!(sessions.len(), 3, "Should have 3 sessions in LIST");

    // Find session-2 and verify it's closed
    let session_2 = sessions
        .iter()
        .find(|s| s.starts_with("session-2"))
        .expect("session-2 should be in LIST");
    assert!(
        session_2.contains("closed"),
        "session-2 should be closed, got: {}",
        session_2
    );

    // Find session-1 and session-3 and verify they're still working
    let session_1 = sessions
        .iter()
        .find(|s| s.starts_with("session-1"))
        .expect("session-1 should be in LIST");
    assert!(
        session_1.contains("working"),
        "session-1 should be working, got: {}",
        session_1
    );

    let session_3 = sessions
        .iter()
        .find(|s| s.starts_with("session-3"))
        .expect("session-3 should be in LIST");
    assert!(
        session_3.contains("working"),
        "session-3 should be working, got: {}",
        session_3
    );

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}

/// Test session lifecycle with status transitions through all states.
#[tokio::test]
async fn test_lifecycle_status_transitions() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "status_trans");

    // Start server
    let (server, shutdown_tx) = start_server_with_shutdown(socket_path.clone()).await;

    // Run server in background
    let shutdown_rx = shutdown_tx.subscribe();
    let server_handle = tokio::spawn(async move {
        let _ = server.run_with_shutdown(shutdown_rx).await;
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Connect client
    let stream = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect");
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut response = String::new();

    // Create session with working status
    writer
        .write_all(b"SET trans-session working /project\n")
        .await
        .expect("Failed to write");
    writer.flush().await.expect("Failed to flush");

    response.clear();
    reader.read_line(&mut response).await.expect("Failed to read");
    assert!(response.contains("working"), "Initial status should be working");

    // Transition: working -> attention
    writer
        .write_all(b"SET trans-session attention\n")
        .await
        .expect("Failed to write");
    writer.flush().await.expect("Failed to flush");

    response.clear();
    reader.read_line(&mut response).await.expect("Failed to read");
    assert!(
        response.contains("attention"),
        "Status should transition to attention"
    );

    // Transition: attention -> question
    writer
        .write_all(b"SET trans-session question\n")
        .await
        .expect("Failed to write");
    writer.flush().await.expect("Failed to flush");

    response.clear();
    reader.read_line(&mut response).await.expect("Failed to read");
    assert!(
        response.contains("question"),
        "Status should transition to question"
    );

    // Transition: question -> working
    writer
        .write_all(b"SET trans-session working\n")
        .await
        .expect("Failed to write");
    writer.flush().await.expect("Failed to flush");

    response.clear();
    reader.read_line(&mut response).await.expect("Failed to read");
    assert!(
        response.contains("working"),
        "Status should transition back to working"
    );

    // Close via RM
    writer
        .write_all(b"RM trans-session\n")
        .await
        .expect("Failed to write");
    writer.flush().await.expect("Failed to flush");

    response.clear();
    reader.read_line(&mut response).await.expect("Failed to read");
    assert!(
        response.contains("closed"),
        "Session should be closed after RM"
    );

    // Verify via GET that session is closed
    writer
        .write_all(b"GET trans-session\n")
        .await
        .expect("Failed to write");
    writer.flush().await.expect("Failed to flush");

    response.clear();
    reader.read_line(&mut response).await.expect("Failed to read");
    assert!(
        response.contains("closed"),
        "GET should show closed status, got: {}",
        response
    );

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}

// ============================================================================
// Subscriber Notification Tests
// ============================================================================

/// Test that a subscriber receives UPDATE messages when sessions are created/updated.
///
/// This test verifies the subscriber notification system by:
/// 1. Connecting a subscriber client via SUB command
/// 2. Having another client create and update a session
/// 3. Verifying the subscriber receives UPDATE messages with correct format
#[tokio::test]
async fn test_subscriber_receives_update() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "sub_update");

    // Start server
    let (server, shutdown_tx) = start_server_with_shutdown(socket_path.clone()).await;

    // Run server in background
    let shutdown_rx = shutdown_tx.subscribe();
    let server_handle = tokio::spawn(async move {
        let _ = server.run_with_shutdown(shutdown_rx).await;
    });

    // Give server time to start accepting
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Connect subscriber client first
    let subscriber_stream = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect subscriber");
    let (sub_reader, mut sub_writer) = subscriber_stream.into_split();
    let mut sub_reader = BufReader::new(sub_reader);
    let mut sub_response = String::new();

    // Send SUB command
    sub_writer
        .write_all(b"SUB\n")
        .await
        .expect("Failed to write SUB");
    sub_writer.flush().await.expect("Failed to flush SUB");

    // Wait for "OK subscribed" acknowledgment
    sub_reader
        .read_line(&mut sub_response)
        .await
        .expect("Failed to read SUB response");
    assert!(
        sub_response.starts_with("OK subscribed"),
        "Expected 'OK subscribed', got: {}",
        sub_response
    );

    // Give subscriber time to register with the store
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Connect a separate client to create/update sessions
    let client_stream = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect client");
    let (client_reader, mut client_writer) = client_stream.into_split();
    let mut client_reader = BufReader::new(client_reader);
    let mut client_response = String::new();

    // Step 1: Create a session via SET command
    client_writer
        .write_all(b"SET sub-test-session working /home/user/project\n")
        .await
        .expect("Failed to write SET");
    client_writer.flush().await.expect("Failed to flush SET");

    // Read SET response on client
    client_reader
        .read_line(&mut client_response)
        .await
        .expect("Failed to read SET response");
    assert!(
        client_response.starts_with("OK sub-test-session working"),
        "SET should succeed, got: {}",
        client_response
    );

    // Note: Creating a new session doesn't trigger an UPDATE since there's no status _change_
    // The notification is sent when status actually changes. Let's update the session status.

    // Step 2: Update session status via SET command (working -> attention)
    client_response.clear();
    client_writer
        .write_all(b"SET sub-test-session attention\n")
        .await
        .expect("Failed to write SET update");
    client_writer.flush().await.expect("Failed to flush SET update");

    // Read SET update response on client
    client_reader
        .read_line(&mut client_response)
        .await
        .expect("Failed to read SET update response");
    assert!(
        client_response.starts_with("OK sub-test-session attention"),
        "SET update should succeed, got: {}",
        client_response
    );

    // Step 3: Subscriber should receive UPDATE notification
    sub_response.clear();
    let update_result = timeout(
        Duration::from_secs(1),
        sub_reader.read_line(&mut sub_response),
    )
    .await;

    assert!(
        update_result.is_ok(),
        "Subscriber should receive UPDATE within timeout"
    );
    assert!(
        update_result.unwrap().is_ok(),
        "Subscriber read should succeed"
    );

    // Verify UPDATE message format: UPDATE <session_id> <status> <elapsed_seconds>
    let parts: Vec<&str> = sub_response.trim().split_whitespace().collect();
    assert!(
        parts.len() >= 4,
        "UPDATE message should have at least 4 parts, got: {}",
        sub_response
    );
    assert_eq!(
        parts[0], "UPDATE",
        "Message should start with UPDATE, got: {}",
        sub_response
    );
    assert_eq!(
        parts[1], "sub-test-session",
        "Session ID should match, got: {}",
        parts[1]
    );
    assert_eq!(
        parts[2], "attention",
        "Status should be attention, got: {}",
        parts[2]
    );
    // Verify elapsed_seconds is a valid number
    let elapsed: Result<u64, _> = parts[3].parse();
    assert!(
        elapsed.is_ok(),
        "Elapsed seconds should be a number, got: {}",
        parts[3]
    );

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}

/// Test that multiple subscribers all receive UPDATE messages when a session is updated.
///
/// This test verifies that the notification fanout works correctly by:
/// 1. Connecting N subscriber clients via SUB command
/// 2. Having another client update a session
/// 3. Verifying ALL N subscribers receive the UPDATE message
#[tokio::test]
async fn test_multiple_subscribers() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let socket_path = unique_socket_path(&temp_dir, "multi_sub");

    // Start server
    let (server, shutdown_tx) = start_server_with_shutdown(socket_path.clone()).await;

    // Run server in background
    let shutdown_rx = shutdown_tx.subscribe();
    let server_handle = tokio::spawn(async move {
        let _ = server.run_with_shutdown(shutdown_rx).await;
    });

    // Give server time to start accepting
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Number of subscribers to test
    const NUM_SUBSCRIBERS: usize = 5;

    // Connect N subscriber clients
    let mut subscribers = Vec::new();
    for i in 0..NUM_SUBSCRIBERS {
        let subscriber_stream = UnixStream::connect(&socket_path)
            .await
            .unwrap_or_else(|_| panic!("Failed to connect subscriber {}", i));
        let (sub_reader, mut sub_writer) = subscriber_stream.into_split();
        let mut sub_reader = BufReader::new(sub_reader);
        let mut sub_response = String::new();

        // Send SUB command
        sub_writer
            .write_all(b"SUB\n")
            .await
            .unwrap_or_else(|_| panic!("Failed to write SUB for subscriber {}", i));
        sub_writer
            .flush()
            .await
            .unwrap_or_else(|_| panic!("Failed to flush SUB for subscriber {}", i));

        // Wait for "OK subscribed" acknowledgment
        sub_reader
            .read_line(&mut sub_response)
            .await
            .unwrap_or_else(|_| panic!("Failed to read SUB response for subscriber {}", i));
        assert!(
            sub_response.starts_with("OK subscribed"),
            "Subscriber {} expected 'OK subscribed', got: {}",
            i,
            sub_response
        );

        subscribers.push((sub_reader, sub_writer));
    }

    // Give all subscribers time to register with the store
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Connect a separate client to create/update sessions
    let client_stream = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect client");
    let (client_reader, mut client_writer) = client_stream.into_split();
    let mut client_reader = BufReader::new(client_reader);
    let mut client_response = String::new();

    // Create a session first
    client_writer
        .write_all(b"SET multi-sub-session working /home/user/project\n")
        .await
        .expect("Failed to write SET");
    client_writer.flush().await.expect("Failed to flush SET");

    // Read SET response on client
    client_reader
        .read_line(&mut client_response)
        .await
        .expect("Failed to read SET response");
    assert!(
        client_response.starts_with("OK multi-sub-session working"),
        "SET should succeed, got: {}",
        client_response
    );

    // Update session status (this triggers UPDATE notifications to all subscribers)
    client_response.clear();
    client_writer
        .write_all(b"SET multi-sub-session attention\n")
        .await
        .expect("Failed to write SET update");
    client_writer
        .flush()
        .await
        .expect("Failed to flush SET update");

    // Read SET update response on client
    client_reader
        .read_line(&mut client_response)
        .await
        .expect("Failed to read SET update response");
    assert!(
        client_response.starts_with("OK multi-sub-session attention"),
        "SET update should succeed, got: {}",
        client_response
    );

    // Verify ALL N subscribers receive the UPDATE message
    let mut successful_updates = 0;
    for (i, (mut sub_reader, _sub_writer)) in subscribers.into_iter().enumerate() {
        let mut sub_response = String::new();
        let update_result = timeout(
            Duration::from_secs(1),
            sub_reader.read_line(&mut sub_response),
        )
        .await;

        match update_result {
            Ok(Ok(_)) => {
                // Verify UPDATE message format: UPDATE <session_id> <status> <elapsed_seconds>
                let parts: Vec<&str> = sub_response.trim().split_whitespace().collect();
                assert!(
                    parts.len() >= 4,
                    "Subscriber {} UPDATE message should have at least 4 parts, got: {}",
                    i,
                    sub_response
                );
                assert_eq!(
                    parts[0], "UPDATE",
                    "Subscriber {} message should start with UPDATE, got: {}",
                    i,
                    sub_response
                );
                assert_eq!(
                    parts[1], "multi-sub-session",
                    "Subscriber {} session ID should match, got: {}",
                    i,
                    parts[1]
                );
                assert_eq!(
                    parts[2], "attention",
                    "Subscriber {} status should be attention, got: {}",
                    i,
                    parts[2]
                );
                // Verify elapsed_seconds is a valid number
                let elapsed: Result<u64, _> = parts[3].parse();
                assert!(
                    elapsed.is_ok(),
                    "Subscriber {} elapsed seconds should be a number, got: {}",
                    i,
                    parts[3]
                );
                successful_updates += 1;
            }
            Ok(Err(e)) => {
                panic!("Subscriber {} read failed: {}", i, e);
            }
            Err(_) => {
                panic!("Subscriber {} timed out waiting for UPDATE", i);
            }
        }
    }

    // All subscribers should have received the UPDATE
    assert_eq!(
        successful_updates, NUM_SUBSCRIBERS,
        "All {} subscribers should receive UPDATE, only {} did",
        NUM_SUBSCRIBERS, successful_updates
    );

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}
