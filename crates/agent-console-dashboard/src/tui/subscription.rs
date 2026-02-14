//! Daemon subscription and message parsing for the TUI.
//!
//! Handles connecting to the daemon, subscribing to live updates (session
//! changes and usage data), and parsing the JSON Lines IPC protocol into typed
//! messages.

use crate::client::connect_with_lazy_start;
use crate::{IpcCommand, IpcNotification, IpcResponse, SessionSnapshot, IPC_VERSION};
use claude_usage::UsageData;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

/// Messages received from the daemon via the SUB subscription.
#[derive(Debug)]
pub enum DaemonMessage {
    /// A session update with full session info.
    SessionUpdate(SessionSnapshot),
    /// Updated API usage data.
    UsageUpdate(UsageData),
}

/// Connects to the daemon via Unix socket, sends LIST to get initial state,
/// then SUB to receive live updates. Sends parsed updates through the channel.
pub async fn subscribe_to_daemon(
    socket_path: &Path,
    tx: mpsc::Sender<DaemonMessage>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = connect_with_lazy_start(socket_path).await?;
    let stream = client.into_stream();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Send LIST command as JSON
    let list_cmd = IpcCommand {
        version: IPC_VERSION,
        cmd: "LIST".to_string(),
        session_id: None,
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };
    let list_json = serde_json::to_string(&list_cmd).expect("failed to serialize LIST command");
    writer.write_all(list_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    let mut line = String::new();
    reader.read_line(&mut line).await?;

    // Parse LIST response as IpcResponse
    if let Ok(resp) = serde_json::from_str::<IpcResponse>(line.trim()) {
        if resp.ok {
            if let Some(data) = resp.data {
                if let Ok(sessions) = serde_json::from_value::<Vec<SessionSnapshot>>(data) {
                    for info in sessions {
                        let _ = tx.send(DaemonMessage::SessionUpdate(info)).await;
                    }
                }
            }
        }
    }

    // Now subscribe for live updates -- need a new connection since LIST consumed the first
    let client = connect_with_lazy_start(socket_path).await?;
    let stream = client.into_stream();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Send SUB command as JSON
    let sub_cmd = IpcCommand {
        version: IPC_VERSION,
        cmd: "SUB".to_string(),
        session_id: None,
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };
    let sub_json = serde_json::to_string(&sub_cmd).expect("failed to serialize SUB command");
    writer.write_all(sub_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    line.clear();
    reader.read_line(&mut line).await?; // IpcResponse {"ok": true, "data": "subscribed"}

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

/// Parses a single JSON line from the daemon SUB stream into a `DaemonMessage`.
///
/// Returns `None` for unrecognized or malformed lines.
pub fn parse_daemon_line(line: &str) -> Option<DaemonMessage> {
    let notification: IpcNotification = match serde_json::from_str(line) {
        Ok(n) => n,
        Err(_) => return None,
    };

    match notification.notification_type.as_str() {
        "update" => {
            let info = notification.session?;
            Some(DaemonMessage::SessionUpdate(info))
        }
        "usage" => {
            let usage_value = notification.usage?;
            match serde_json::from_value::<UsageData>(usage_value) {
                Ok(data) => Some(DaemonMessage::UsageUpdate(data)),
                Err(e) => {
                    tracing::warn!("failed to parse usage data: {}", e);
                    None
                }
            }
        }
        "warn" => {
            if let Some(msg) = notification.message {
                tracing::warn!("daemon warning: {}", msg);
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IPC_VERSION;

    fn make_update_notification(session_id: &str, status: &str) -> String {
        let info = SessionSnapshot {
            session_id: session_id.to_string(),
            agent_type: "claudecode".to_string(),
            status: status.to_string(),
            working_dir: Some("/tmp/test".to_string()),
            elapsed_seconds: 120,
            idle_seconds: 5,
            history: vec![],
            closed: false,
            priority: 0,
        };
        let notification = IpcNotification::session_update(info);
        serde_json::to_string(&notification).expect("failed to serialize notification")
    }

    #[test]
    fn test_parse_update_message() {
        let json = make_update_notification("session-1", "working");
        let msg = parse_daemon_line(&json);
        match msg {
            Some(DaemonMessage::SessionUpdate(info)) => {
                assert_eq!(info.session_id, "session-1");
                assert_eq!(info.status, "working");
                assert_eq!(info.elapsed_seconds, 120);
                assert_eq!(info.working_dir, Some("/tmp/test".to_string()));
            }
            other => panic!("expected SessionUpdate, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_usage_message() {
        let data = claude_usage::UsageData {
            five_hour: claude_usage::UsagePeriod {
                utilization: 25.0,
                resets_at: None,
            },
            seven_day: claude_usage::UsagePeriod {
                utilization: 50.0,
                resets_at: None,
            },
            seven_day_sonnet: None,
            extra_usage: None,
        };
        let notification = IpcNotification::usage_update(&data);
        let json = serde_json::to_string(&notification).expect("failed to serialize");
        let msg = parse_daemon_line(&json);
        match msg {
            Some(DaemonMessage::UsageUpdate(data)) => {
                assert!((data.five_hour.utilization - 25.0).abs() < f64::EPSILON);
                assert!((data.seven_day.utilization - 50.0).abs() < f64::EPSILON);
            }
            other => panic!("expected UsageUpdate, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_warn_message() {
        let notification = IpcNotification {
            version: IPC_VERSION,
            notification_type: "warn".to_string(),
            session: None,
            usage: None,
            message: Some("lagged 5".to_string()),
        };
        let json = serde_json::to_string(&notification).expect("failed to serialize");
        // Warn messages return None (they're logged, not forwarded)
        assert!(parse_daemon_line(&json).is_none());
    }

    #[test]
    fn test_parse_empty_line_returns_none() {
        assert!(parse_daemon_line("").is_none());
    }

    #[test]
    fn test_parse_invalid_json_returns_none() {
        assert!(parse_daemon_line("{invalid json}").is_none());
    }

    #[test]
    fn test_parse_unknown_notification_type_returns_none() {
        let json = r#"{"version":1,"type":"unknown","session":null,"usage":null,"message":null}"#;
        assert!(parse_daemon_line(json).is_none());
    }
}
