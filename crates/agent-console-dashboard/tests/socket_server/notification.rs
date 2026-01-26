//! Subscriber notification tests for Unix Socket Server.

use super::common::{start_server_with_shutdown, unique_socket_path};
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::timeout;

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
        update_result
            .expect("already checked is_ok")
            .is_ok(),
        "Subscriber read should succeed"
    );

    // Verify UPDATE message format: UPDATE <session_id> <status> <elapsed_seconds>
    let parts: Vec<&str> = sub_response.split_whitespace().collect();
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
                let parts: Vec<&str> = sub_response.split_whitespace().collect();
                assert!(
                    parts.len() >= 4,
                    "Subscriber {} UPDATE message should have at least 4 parts, got: {}",
                    i,
                    sub_response
                );
                assert_eq!(
                    parts[0], "UPDATE",
                    "Subscriber {} message should start with UPDATE, got: {}",
                    i, sub_response
                );
                assert_eq!(
                    parts[1], "multi-sub-session",
                    "Subscriber {} session ID should match, got: {}",
                    i, parts[1]
                );
                assert_eq!(
                    parts[2], "attention",
                    "Subscriber {} status should be attention, got: {}",
                    i, parts[2]
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
