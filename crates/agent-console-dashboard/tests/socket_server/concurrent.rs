//! Concurrent client tests for Unix Socket Server.

use super::common::{start_server_with_shutdown, unique_socket_path};
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::timeout;

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
            let stream = UnixStream::connect(&path).await.expect("Failed to connect");
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
        let _ = result
            .expect("already checked is_ok")
            .expect("Client task should succeed");
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
    writer.write_all(b"LIST\n").await.expect("Should write");
    writer.flush().await.expect("Should flush");

    let mut response = String::new();
    reader.read_line(&mut response).await.expect("Should read");
    assert!(
        response.starts_with("OK"),
        "Server should respond with OK, got: {}",
        response
    );

    // Shutdown
    shutdown_tx.send(()).ok();
    let _ = timeout(Duration::from_secs(1), server_handle).await;
}
