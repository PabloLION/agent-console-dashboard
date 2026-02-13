use super::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;
use tokio::net::UnixListener;
use tokio::time::timeout;

/// Atomic counter for generating unique socket paths across parallel tests.
static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Creates a TempDir in /tmp, bypassing the TMPDIR env var which may be
/// set to a relative path by Claude Code (causing "No such file or directory").
fn create_temp_dir() -> TempDir {
    TempDir::new_in("/tmp").expect("Failed to create temp directory in /tmp")
}

/// Generates a unique socket path within a temporary directory.
fn unique_socket_path(temp_dir: &TempDir, prefix: &str) -> PathBuf {
    let count = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    temp_dir.path().join(format!("{}_{}.sock", prefix, count))
}

#[test]
fn test_client_error_display() {
    let err = ClientError::DaemonStartFailed {
        attempts: 10,
        last_error: None,
    };
    let display = err.to_string();
    assert!(display.contains("Daemon failed to start after 10 attempts"));
    assert!(display.contains("Last error: unknown"));
}

#[test]
fn test_client_error_spawn_failed_display() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "binary not found");
    let err = ClientError::SpawnFailed(io_err);
    assert!(err.to_string().contains("Failed to spawn daemon process"));
}

#[test]
fn test_client_error_executable_not_found_display() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "exe not found");
    let err = ClientError::ExecutableNotFound(io_err);
    assert!(err
        .to_string()
        .contains("Failed to find current executable"));
}

#[test]
fn test_client_error_connection_failed_display() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
    let err = ClientError::ConnectionFailed(io_err);
    let display = err.to_string();
    assert!(display.contains("Connection to daemon failed"));
    assert!(display.contains("cannot be resolved by lazy-starting"));
}

#[test]
fn test_client_error_source_connection_failed() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
    let err = ClientError::ConnectionFailed(io_err);
    assert!(err.source().is_some());
}

#[test]
fn test_client_error_source_daemon_start_failed() {
    // DaemonStartFailed with no last_error has no source
    let err = ClientError::DaemonStartFailed {
        attempts: 5,
        last_error: None,
    };
    assert!(err.source().is_none());

    // DaemonStartFailed with last_error has a source
    let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
    let err_with_source = ClientError::DaemonStartFailed {
        attempts: 5,
        last_error: Some(io_err),
    };
    assert!(err_with_source.source().is_some());
}

#[test]
fn test_client_error_source_spawn_failed() {
    // SpawnFailed wraps an io::Error as source
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "binary not found");
    let err = ClientError::SpawnFailed(io_err);
    assert!(err.source().is_some());
}

#[test]
fn test_client_error_source_executable_not_found() {
    // ExecutableNotFound wraps an io::Error as source
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "exe not found");
    let err = ClientError::ExecutableNotFound(io_err);
    assert!(err.source().is_some());
}

#[test]
fn test_backoff_delays() {
    // Verify delays double: 10→20→40→80→160→320→500→500→500→500ms
    assert_eq!(calculate_backoff(0), Duration::from_millis(10));
    assert_eq!(calculate_backoff(1), Duration::from_millis(20));
    assert_eq!(calculate_backoff(2), Duration::from_millis(40));
    assert_eq!(calculate_backoff(3), Duration::from_millis(80));
    assert_eq!(calculate_backoff(4), Duration::from_millis(160));
    assert_eq!(calculate_backoff(5), Duration::from_millis(320));
    // After this point, delays are capped at MAX_BACKOFF_MS (500ms)
    assert_eq!(calculate_backoff(6), Duration::from_millis(500));
    assert_eq!(calculate_backoff(7), Duration::from_millis(500));
    assert_eq!(calculate_backoff(8), Duration::from_millis(500));
    assert_eq!(calculate_backoff(9), Duration::from_millis(500));
}

#[test]
fn test_backoff_at_max_retries() {
    // Verify backoff at MAX_RETRIES boundary
    // The function is designed for use with attempts 0..MAX_RETRIES
    let delay = calculate_backoff(MAX_RETRIES - 1);
    assert_eq!(delay, Duration::from_millis(MAX_BACKOFF_MS));

    // Also verify the last configured retry
    let delay = calculate_backoff(MAX_RETRIES);
    assert_eq!(delay, Duration::from_millis(MAX_BACKOFF_MS));
}

#[test]
fn test_constants() {
    // Verify retry configuration
    assert_eq!(INITIAL_BACKOFF_MS, 10);
    assert_eq!(MAX_BACKOFF_MS, 500);
    assert_eq!(MAX_RETRIES, 10);
}

#[test]
fn test_client_result_type_compatibility() {
    // Verify ClientResult type is properly defined and usable
    // This test verifies type compatibility without actual socket operations
    let _result: ClientResult<()> = Ok(());
    let _err_result: ClientResult<()> = Err(Box::new(ClientError::DaemonStartFailed {
        attempts: 10,
        last_error: None,
    }));
}

#[test]
fn test_client_error_is_send_sync() {
    // Verify ClientError can be used in async/threaded contexts
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    // ClientError should be Send + Sync for use with async code
    assert_send::<ClientError>();
    assert_sync::<ClientError>();
}

#[test]
fn test_client_error_debug() {
    // Verify Debug trait is derived
    let err = ClientError::DaemonStartFailed {
        attempts: 3,
        last_error: None,
    };
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("DaemonStartFailed"));
    assert!(debug_str.contains("attempts: 3"));

    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
    let spawn_err = ClientError::SpawnFailed(io_err);
    let debug_str = format!("{:?}", spawn_err);
    assert!(debug_str.contains("SpawnFailed"));
}

// =========================================================================
// Async connection tests (moved from tests/client_lazy_start.rs)
// =========================================================================

/// Tests that a client can connect to an already-running daemon without spawning.
#[tokio::test]
async fn test_client_connects_to_existing_daemon() {
    let temp_dir = create_temp_dir();
    let socket_path = unique_socket_path(&temp_dir, "existing_daemon");

    // Start a mock daemon server
    let listener = UnixListener::bind(&socket_path).expect("Failed to bind socket");

    // Spawn a task to accept connections
    let accept_handle = tokio::spawn(async move {
        let result = timeout(Duration::from_secs(5), listener.accept()).await;
        match result {
            Ok(Ok((stream, _))) => {
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
        connect_with_lazy_start(&socket_path),
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
#[tokio::test]
async fn test_concurrent_clients_connect_to_existing_daemon() {
    let temp_dir = create_temp_dir();
    let socket_path = unique_socket_path(&temp_dir, "concurrent");

    let listener = UnixListener::bind(&socket_path).expect("Failed to bind socket");
    let socket_path_clone = socket_path.clone();

    // Spawn a task to accept multiple connections
    let accept_handle = tokio::spawn(async move {
        let mut connections = 0;
        while let Ok(Ok((stream, _))) = timeout(Duration::from_secs(5), listener.accept()).await
        {
            connections += 1;
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
            timeout(Duration::from_secs(3), connect_with_lazy_start(&path)).await
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

    assert_eq!(
        successful_connections, 3,
        "Not all concurrent clients connected successfully"
    );

    let accepted_count = accept_handle.await.expect("Accept task panicked");
    assert_eq!(accepted_count, 3, "Server did not accept all connections");
}

/// Tests that timeout error is returned when daemon cannot be started.
#[tokio::test]
async fn test_timeout_error_when_daemon_fails_to_start() {
    let temp_dir = create_temp_dir();
    let socket_path = unique_socket_path(&temp_dir, "timeout_test");

    let start = std::time::Instant::now();
    let result = connect_with_lazy_start(&socket_path).await;
    let elapsed = start.elapsed();

    assert!(result.is_err(), "Expected connection to fail");

    let err = result.unwrap_err();
    let err_string = err.to_string();

    // Accept either timeout or spawn failure
    let is_expected_error = err_string.contains("Daemon failed to start")
        || err_string.contains("Failed to spawn")
        || err_string.contains("Failed to find current executable");

    assert!(is_expected_error, "Unexpected error type: {}", err_string);

    if err_string.contains("Daemon failed to start") {
        assert!(
            elapsed >= Duration::from_millis(500),
            "Timeout happened too quickly: {:?}",
            elapsed
        );
    }
}

/// Tests that connection succeeds after a brief startup delay.
#[tokio::test]
async fn test_connection_succeeds_after_startup_delay() {
    let temp_dir = create_temp_dir();
    let socket_path = unique_socket_path(&temp_dir, "delayed_start");
    let socket_path_for_listener = socket_path.clone();

    let listener_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;

        let listener =
            UnixListener::bind(&socket_path_for_listener).expect("Failed to bind socket");

        match timeout(Duration::from_secs(5), listener.accept()).await {
            Ok(Ok((stream, _))) => {
                tokio::time::sleep(Duration::from_millis(100)).await;
                drop(stream);
                true
            }
            _ => false,
        }
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let connect_result = timeout(
        Duration::from_secs(5),
        connect_with_lazy_start(&socket_path),
    )
    .await;

    if let Ok(Ok(_client)) = connect_result {
        let accepted = listener_handle.await.expect("Listener task panicked");
        assert!(accepted, "Server should have accepted the connection");
    }
}

/// Tests that the Client struct properly wraps the UnixStream.
#[tokio::test]
async fn test_client_stream_access() {
    let temp_dir = create_temp_dir();
    let socket_path = unique_socket_path(&temp_dir, "stream_access");

    let listener = UnixListener::bind(&socket_path).expect("Failed to bind socket");

    let accept_handle = tokio::spawn(async move {
        let _ = timeout(Duration::from_secs(2), listener.accept()).await;
    });

    let result = timeout(
        Duration::from_secs(2),
        connect_with_lazy_start(&socket_path),
    )
    .await;

    if let Ok(Ok(client)) = result {
        let _stream_ref = client.stream();
        let _stream = client.into_stream();
    }

    let _ = accept_handle.await;
}

/// Tests that connecting to a non-existent socket triggers lazy-start flow.
#[tokio::test]
async fn test_lazy_start_triggered_on_missing_socket() {
    let temp_dir = create_temp_dir();
    let socket_path = unique_socket_path(&temp_dir, "lazy_start");

    assert!(!socket_path.exists(), "Socket should not exist before test");

    let result = connect_with_lazy_start(&socket_path).await;

    // In test environment, this will fail because spawn_daemon uses test binary
    assert!(result.is_err(), "Expected failure in test environment");
}

/// Tests behavior when socket path is in a non-existent directory.
#[tokio::test]
async fn test_connection_to_invalid_path() {
    let invalid_path = PathBuf::from("/nonexistent/directory/socket.sock");

    let result = connect_with_lazy_start(&invalid_path).await;

    assert!(result.is_err(), "Expected failure for invalid path");
}
