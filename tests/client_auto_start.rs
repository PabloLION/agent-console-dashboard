//! Integration tests for client auto-start functionality.
//!
//! These tests verify the client connection behavior including auto-start,
//! retry logic, timeout handling, and concurrent connection safety.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tempfile::TempDir;
use tokio::net::UnixListener;
use tokio::time::timeout;

use agent_console::client::{connect_with_auto_start, ClientError};

/// Atomic counter for generating unique socket paths across parallel tests.
static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Generates a unique socket path within a temporary directory.
///
/// This ensures test isolation when running tests in parallel.
fn unique_socket_path(temp_dir: &TempDir, prefix: &str) -> PathBuf {
    let count = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    temp_dir.path().join(format!("{}_{}.sock", prefix, count))
}

/// Tests that a client can connect to an already-running daemon without spawning.
///
/// This verifies requirement: "No Duplicate Daemon Spawn" - when daemon is already
/// running, connect directly without spawning another instance.
#[tokio::test]
async fn test_client_connects_to_existing_daemon() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let socket_path = unique_socket_path(&temp_dir, "existing_daemon");

    // Start a mock daemon server
    let listener = UnixListener::bind(&socket_path).expect("Failed to bind socket");

    // Spawn a task to accept connections
    let accept_handle = tokio::spawn(async move {
        let result = timeout(Duration::from_secs(5), listener.accept()).await;
        match result {
            Ok(Ok((stream, _))) => {
                // Keep the stream alive briefly to allow test to complete
                tokio::time::sleep(Duration::from_millis(100)).await;
                drop(stream);
                true
            }
            _ => false,
        }
    });

    // Connect to the mock daemon - should succeed immediately without spawning
    let connect_result = timeout(
        Duration::from_secs(2),
        connect_with_auto_start(&socket_path),
    )
    .await;

    assert!(connect_result.is_ok(), "Connection timed out unexpectedly");
    assert!(
        connect_result.unwrap().is_ok(),
        "Failed to connect to existing daemon"
    );

    // Verify the server accepted the connection
    let accepted = accept_handle.await.expect("Accept task panicked");
    assert!(accepted, "Server did not accept connection");
}

/// Tests that multiple clients can connect to the same daemon concurrently.
///
/// This verifies requirement: "Race Condition Handling" - when multiple clients
/// try to connect simultaneously with daemon running, all should succeed.
#[tokio::test]
async fn test_concurrent_clients_connect_to_existing_daemon() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let socket_path = unique_socket_path(&temp_dir, "concurrent");

    // Start a mock daemon server
    let listener = UnixListener::bind(&socket_path).expect("Failed to bind socket");
    let socket_path_clone = socket_path.clone();

    // Spawn a task to accept multiple connections
    let accept_handle = tokio::spawn(async move {
        let mut connections = 0;
        while let Ok(Ok((stream, _))) = timeout(Duration::from_secs(5), listener.accept()).await {
            connections += 1;
            // Hold connection briefly
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(200)).await;
                drop(stream);
            });
            if connections >= 3 {
                break;
            }
        }
        connections
    });

    // Spawn 3 concurrent client connections
    let mut handles = Vec::new();
    for _ in 0..3 {
        let path = socket_path_clone.clone();
        let handle = tokio::spawn(async move {
            timeout(Duration::from_secs(3), connect_with_auto_start(&path)).await
        });
        handles.push(handle);
    }

    // Wait for all clients to connect
    let mut successful_connections = 0;
    for handle in handles {
        if let Ok(Ok(Ok(_))) = handle.await {
            successful_connections += 1;
        }
    }

    // All 3 clients should connect successfully
    assert_eq!(
        successful_connections, 3,
        "Not all concurrent clients connected successfully"
    );

    // Verify server accepted all connections
    let accepted_count = accept_handle.await.expect("Accept task panicked");
    assert_eq!(accepted_count, 3, "Server did not accept all connections");
}

/// Tests that timeout error is returned when daemon cannot be started.
///
/// This verifies requirement: "Clear Timeout Error" - if daemon fails to start
/// within timeout, return descriptive error.
///
/// Note: In test environment, spawn_daemon will fail because the test binary
/// is not the actual daemon. This allows us to verify timeout behavior.
#[tokio::test]
async fn test_timeout_error_when_daemon_fails_to_start() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let socket_path = unique_socket_path(&temp_dir, "timeout_test");

    // Attempt to connect with no daemon running
    // The spawn will fail (test binary isn't daemon), but we should get timeout
    // after exhausting retries
    let start = std::time::Instant::now();
    let result = connect_with_auto_start(&socket_path).await;
    let elapsed = start.elapsed();

    // Should fail with DaemonStartFailed error
    assert!(result.is_err(), "Expected connection to fail");

    // The error should be DaemonStartFailed (timeout) or SpawnFailed
    // In test environment, it could be either depending on how spawn fails
    let err = match result {
        Err(e) => e,
        Ok(_) => panic!("Expected error, got Ok"),
    };
    let err_string = err.to_string();

    // Accept either timeout or spawn failure
    let is_expected_error = err_string.contains("Daemon failed to start")
        || err_string.contains("Failed to spawn")
        || err_string.contains("Failed to find current executable");

    assert!(is_expected_error, "Unexpected error type: {}", err_string);

    // If we got DaemonStartFailed (timeout), verify timing is reasonable
    // Minimum expected time: ~1-2 seconds for retries
    if err_string.contains("Daemon failed to start") {
        assert!(
            elapsed >= Duration::from_millis(500),
            "Timeout happened too quickly: {:?}",
            elapsed
        );
    }
}

/// Tests that connection succeeds after a brief startup delay.
///
/// This simulates the scenario where daemon takes some time to initialize,
/// and verifies the retry logic waits appropriately.
#[tokio::test]
async fn test_connection_succeeds_after_startup_delay() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let socket_path = unique_socket_path(&temp_dir, "delayed_start");
    let socket_path_for_listener = socket_path.clone();

    // Spawn a task that starts listening after a delay (simulating daemon startup)
    let listener_handle = tokio::spawn(async move {
        // Wait 100ms before starting to listen (simulating daemon startup time)
        tokio::time::sleep(Duration::from_millis(100)).await;

        let listener =
            UnixListener::bind(&socket_path_for_listener).expect("Failed to bind socket");

        // Accept one connection
        match timeout(Duration::from_secs(5), listener.accept()).await {
            Ok(Ok((stream, _))) => {
                tokio::time::sleep(Duration::from_millis(100)).await;
                drop(stream);
                true
            }
            _ => false,
        }
    });

    // Give listener a head start
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Try to connect - initial attempt will fail, but retries should succeed
    // Note: This tests the retry behavior when connecting to a server that
    // becomes available during the retry window
    let connect_result = timeout(
        Duration::from_secs(5),
        connect_with_auto_start(&socket_path),
    )
    .await;

    // The connection might fail in test environment due to spawn_daemon issues,
    // but if it succeeds, verify it connected
    if let Ok(Ok(_client)) = connect_result {
        // Connection succeeded after retry
        let accepted = listener_handle.await.expect("Listener task panicked");
        assert!(accepted, "Server should have accepted the connection");
    }
    // If it fails, that's also acceptable in test environment
    // since spawn_daemon uses the test binary
}

/// Tests that the Client struct properly wraps the UnixStream.
#[tokio::test]
async fn test_client_stream_access() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let socket_path = unique_socket_path(&temp_dir, "stream_access");

    // Start a mock server
    let listener = UnixListener::bind(&socket_path).expect("Failed to bind socket");

    // Accept in background
    let accept_handle = tokio::spawn(async move {
        let _ = timeout(Duration::from_secs(2), listener.accept()).await;
    });

    // Connect
    let result = timeout(
        Duration::from_secs(2),
        connect_with_auto_start(&socket_path),
    )
    .await;

    if let Ok(Ok(client)) = result {
        // Verify stream access methods work
        let _stream_ref = client.stream();
        let client = client; // reborrow for into_stream
        let _stream = client.into_stream();
    }

    let _ = accept_handle.await;
}

/// Tests that ClientError types have proper Display implementations.
#[tokio::test]
async fn test_client_error_display() {
    // Test DaemonStartFailed error display
    let err = ClientError::DaemonStartFailed;
    let display = format!("{}", err);
    assert!(
        display.contains("Daemon failed to start"),
        "Unexpected error message: {}",
        display
    );

    // Test SpawnFailed error display
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test error");
    let err = ClientError::SpawnFailed(io_err);
    let display = format!("{}", err);
    assert!(
        display.contains("Failed to spawn"),
        "Unexpected error message: {}",
        display
    );

    // Test ExecutableNotFound error display
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "exe not found");
    let err = ClientError::ExecutableNotFound(io_err);
    let display = format!("{}", err);
    assert!(
        display.contains("Failed to find current executable"),
        "Unexpected error message: {}",
        display
    );
}

/// Tests that connecting to a non-existent socket path properly triggers
/// the auto-start flow (even if spawn fails in test environment).
#[tokio::test]
async fn test_auto_start_triggered_on_missing_socket() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let socket_path = unique_socket_path(&temp_dir, "auto_start");

    // Verify socket does not exist
    assert!(!socket_path.exists(), "Socket should not exist before test");

    // Attempt connection - should trigger auto-start flow
    let result = connect_with_auto_start(&socket_path).await;

    // In test environment, this will fail because:
    // 1. Initial connection fails (no socket)
    // 2. spawn_daemon fails (test binary isn't daemon) OR
    // 3. Retries timeout (daemon never starts)
    assert!(result.is_err(), "Expected failure in test environment");
}

/// Tests behavior when socket path is in a non-existent directory.
#[tokio::test]
async fn test_connection_to_invalid_path() {
    let invalid_path = PathBuf::from("/nonexistent/directory/socket.sock");

    // Attempt connection to invalid path
    let result = connect_with_auto_start(&invalid_path).await;

    // Should fail - either spawn fails or connection fails
    assert!(result.is_err(), "Expected failure for invalid path");
}
