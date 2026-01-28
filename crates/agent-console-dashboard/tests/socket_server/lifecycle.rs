//! Session lifecycle integration tests for Unix Socket Server.

use super::common::{start_server_with_shutdown, unique_socket_path};
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::timeout;

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
    assert!(
        response.starts_with("OK"),
        "LIST header failed: {}",
        response
    );

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
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read");
    assert!(
        response.contains("working"),
        "Initial status should be working"
    );

    // Transition: working -> attention
    writer
        .write_all(b"SET trans-session attention\n")
        .await
        .expect("Failed to write");
    writer.flush().await.expect("Failed to flush");

    response.clear();
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read");
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
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read");
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
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read");
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
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read");
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
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read");
    assert!(
        response.contains("closed"),
        "GET should show closed status, got: {}",
        response
    );

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}
