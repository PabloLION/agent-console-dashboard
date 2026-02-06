//! Daemon subscription and message parsing for the TUI.
//!
//! Handles connecting to the daemon, subscribing to live updates (session
//! changes and usage data), and parsing the SUB wire protocol into typed
//! messages.

use crate::client::connect_with_auto_start;
use crate::Status;
use claude_usage::UsageData;
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

/// Messages received from the daemon via the SUB subscription.
#[derive(Debug)]
pub enum DaemonMessage {
    /// A session status update.
    SessionUpdate {
        /// Session identifier.
        session_id: String,
        /// New session status.
        status: Status,
    },
    /// Updated API usage data.
    UsageUpdate(UsageData),
}

/// Connects to the daemon via Unix socket, sends LIST to get initial state,
/// then SUB to receive live updates. Sends parsed updates through the channel.
pub async fn subscribe_to_daemon(
    socket_path: &Path,
    tx: mpsc::Sender<DaemonMessage>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = connect_with_auto_start(socket_path).await?;
    let stream = client.into_stream();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Fetch initial session list
    writer.write_all(b"LIST\n").await?;
    writer.flush().await?;

    let mut line = String::new();
    reader.read_line(&mut line).await?; // "OK\n" header
    if line.trim() == "OK" {
        // Read session lines until empty line or next command
        loop {
            line.clear();
            // Use a short timeout to detect end of LIST response
            match tokio::time::timeout(Duration::from_millis(100), reader.read_line(&mut line)).await
            {
                Ok(Ok(0)) => break,
                Ok(Ok(_)) => {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(status) = parts[1].parse::<Status>() {
                            let msg = DaemonMessage::SessionUpdate {
                                session_id: parts[0].to_string(),
                                status,
                            };
                            let _ = tx.send(msg).await;
                        }
                    }
                }
                _ => break,
            }
        }
    }

    // Now subscribe for live updates — need a new connection since LIST consumed the first
    let client = connect_with_auto_start(socket_path).await?;
    let stream = client.into_stream();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    writer.write_all(b"SUB\n").await?;
    writer.flush().await?;

    line.clear();
    reader.read_line(&mut line).await?; // "OK subscribed\n"

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).await?;
        if bytes == 0 {
            break;
        }
        let trimmed = line.trim();

        if let Some(msg) = parse_daemon_line(trimmed) {
            if tx.send(msg).await.is_err() {
                break; // receiver dropped
            }
        }
    }

    Ok(())
}

/// Parses a single line from the daemon SUB stream into a `DaemonMessage`.
///
/// Returns `None` for unrecognized or malformed lines (WARN, etc.).
pub fn parse_daemon_line(line: &str) -> Option<DaemonMessage> {
    if let Some(json) = line.strip_prefix("USAGE ") {
        match serde_json::from_str::<UsageData>(json) {
            Ok(data) => return Some(DaemonMessage::UsageUpdate(data)),
            Err(e) => {
                tracing::warn!("failed to parse USAGE message: {}", e);
                return None;
            }
        }
    }

    let parts: Vec<&str> = line.split_whitespace().collect();
    // UPDATE <session_id> <status> <elapsed>
    if parts.len() >= 3 && parts[0] == "UPDATE" {
        match parts[2].parse::<Status>() {
            Ok(status) => {
                return Some(DaemonMessage::SessionUpdate {
                    session_id: parts[1].to_string(),
                    status,
                });
            }
            Err(_) => {
                tracing::warn!("failed to parse UPDATE status: {}", parts[2]);
                return None;
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_update_message() {
        let msg = parse_daemon_line("UPDATE session-1 working 120");
        match msg {
            Some(DaemonMessage::SessionUpdate { session_id, status }) => {
                assert_eq!(session_id, "session-1");
                assert_eq!(status, Status::Working);
            }
            other => panic!("expected SessionUpdate, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_usage_message() {
        let json = r#"USAGE {"five_hour":{"utilization":25.0},"seven_day":{"utilization":50.0}}"#;
        let msg = parse_daemon_line(json);
        match msg {
            Some(DaemonMessage::UsageUpdate(data)) => {
                assert!((data.five_hour.utilization - 25.0).abs() < f64::EPSILON);
                assert!((data.seven_day.utilization - 50.0).abs() < f64::EPSILON);
            }
            other => panic!("expected UsageUpdate, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_unknown_line_returns_none() {
        assert!(parse_daemon_line("WARN lagged 5").is_none());
    }

    #[test]
    fn test_parse_empty_line_returns_none() {
        assert!(parse_daemon_line("").is_none());
    }

    #[test]
    fn test_parse_malformed_usage_returns_none() {
        assert!(parse_daemon_line("USAGE {invalid json}").is_none());
    }

    #[test]
    fn test_parse_update_invalid_status_returns_none() {
        assert!(parse_daemon_line("UPDATE session-1 invalid_status 120").is_none());
    }
}
