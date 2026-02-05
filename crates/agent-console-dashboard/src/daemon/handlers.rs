//! Command handlers for the daemon socket protocol.
//!
//! Each `handle_*` function processes a single command received from a client
//! connection and returns a response string (or streams data for SUB).

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::io::AsyncWriteExt;
use tokio::sync::broadcast;

use crate::daemon::store::SessionStore;
use crate::daemon::usage::UsageFetcher;
use crate::{
    get_memory_usage_mb, AgentType, DaemonDump, HealthStatus, SessionCounts, SessionSnapshot,
    Status,
};

/// Shared daemon state passed to each client handler.
#[derive(Clone)]
pub(super) struct DaemonState {
    pub(super) store: SessionStore,
    pub(super) start_time: Instant,
    pub(super) active_connections: Arc<AtomicUsize>,
    pub(super) socket_path: String,
    pub(super) usage_fetcher: Option<Arc<UsageFetcher>>,
}

/// Handles the SET command: SET <session_id> <status> [working_dir]
///
/// Creates a new session if it doesn't exist, or updates the status if it does.
pub(super) async fn handle_set_command(args: &[&str], store: &SessionStore) -> String {
    if args.len() < 2 {
        return "ERR SET requires: <session_id> <status> [working_dir]\n".to_string();
    }

    let session_id = args[0];
    let status_str = args[1];
    let working_dir = if args.len() > 2 {
        PathBuf::from(args[2])
    } else {
        PathBuf::from("unknown")
    };

    let status: Status = match status_str.parse() {
        Ok(s) => s,
        Err(_) => {
            return format!(
                "ERR invalid status: {} (expected: working, attention, question, closed)\n",
                status_str
            );
        }
    };

    let session = store
        .get_or_create_session(
            session_id.to_string(),
            AgentType::ClaudeCode,
            working_dir,
            None,
            status,
        )
        .await;

    format!("OK {} {}\n", session.id, session.status)
}

/// Handles the RM command: RM <session_id>
///
/// Closes the session (marks as closed, doesn't remove from store).
pub(super) async fn handle_rm_command(args: &[&str], store: &SessionStore) -> String {
    if args.is_empty() {
        return "ERR RM requires: <session_id>\n".to_string();
    }

    let session_id = args[0];

    match store.close_session(session_id).await {
        Some(session) => format!("OK {} closed\n", session.id),
        None => format!("ERR session not found: {}\n", session_id),
    }
}

/// Handles the LIST command: LIST
///
/// Returns all sessions in the format: OK\n<session_id> <status> <elapsed_seconds>\n...
pub(super) async fn handle_list_command(store: &SessionStore) -> String {
    let sessions = store.list_all().await;

    if sessions.is_empty() {
        return "OK\n".to_string();
    }

    let mut response = String::from("OK\n");
    for session in sessions {
        let elapsed = session.since.elapsed().as_secs();
        response.push_str(&format!("{} {} {}\n", session.id, session.status, elapsed));
    }

    response
}

/// Handles the GET command: GET <session_id>
///
/// Returns a single session in the format: OK <session_id> <status> <elapsed_seconds> <working_dir>
pub(super) async fn handle_get_command(args: &[&str], store: &SessionStore) -> String {
    if args.is_empty() {
        return "ERR GET requires: <session_id>\n".to_string();
    }

    let session_id = args[0];

    match store.get(session_id).await {
        Some(session) => {
            let elapsed = session.since.elapsed().as_secs();
            format!(
                "OK {} {} {} {}\n",
                session.id,
                session.status,
                elapsed,
                session.working_dir.display()
            )
        }
        None => format!("ERR session not found: {}\n", session_id),
    }
}

/// Handles the SUB command: SUB
///
/// Subscribes to session updates and usage updates, sending both to the client.
///
/// Wire format:
/// - Session updates: `UPDATE <session_id> <status> <elapsed_seconds>\n`
/// - Usage updates: `USAGE <json>\n`
/// - Lag warnings: `WARN lagged <count>\n`
///
/// On initial subscription, sends the current usage state (if available) as
/// the first USAGE message so clients don't have to wait for the next fetch.
///
/// This function runs until the client disconnects or an error occurs.
pub(super) async fn handle_sub_command(
    store: &SessionStore,
    usage_fetcher: Option<&Arc<UsageFetcher>>,
    writer: &mut tokio::net::unix::OwnedWriteHalf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    writer.write_all(b"OK subscribed\n").await?;
    writer.flush().await?;

    let mut session_rx = store.subscribe();

    // Subscribe to usage updates if fetcher is available
    let mut usage_sub = usage_fetcher.map(|f| f.subscribe());

    // Send current usage state as initial snapshot.
    // Clone data and drop lock before I/O to avoid holding RwLock during writes.
    if let Some(fetcher) = usage_fetcher {
        let usage_state = fetcher.state();
        let snapshot = {
            let guard = usage_state.read().await;
            match &*guard {
                super::usage::UsageState::Available(data) => Some(data.clone()),
                super::usage::UsageState::Unavailable => None,
            }
        };
        if let Some(data) = snapshot {
            let json = serde_json::to_string(&data).expect("failed to serialize UsageData");
            let message = format!("USAGE {}\n", json);
            writer.write_all(message.as_bytes()).await?;
            writer.flush().await?;
        }
    }

    tracing::debug!("Client subscribed to session and usage updates");

    loop {
        // If we have a usage subscription, select on both channels.
        // Otherwise, only listen to session updates.
        if let Some(ref mut usage) = usage_sub {
            tokio::select! {
                result = session_rx.recv() => {
                    match result {
                        Ok(update) => {
                            let message = format!(
                                "UPDATE {} {} {}\n",
                                update.session_id, update.status, update.elapsed_seconds
                            );
                            if write_or_disconnect(writer, &message).await {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::debug!("Session subscriber channel closed");
                            break;
                        }
                        Err(broadcast::error::RecvError::Lagged(count)) => {
                            tracing::warn!("Session subscriber lagged, missed {} messages", count);
                            let lag_message = format!("WARN lagged {}\n", count);
                            if write_or_disconnect(writer, &lag_message).await {
                                break;
                            }
                        }
                    }
                }
                result = usage.recv() => {
                    match result {
                        Ok(super::usage::UsageState::Available(data)) => {
                            let json = serde_json::to_string(&data)
                                .expect("failed to serialize UsageData");
                            let message = format!("USAGE {}\n", json);
                            if write_or_disconnect(writer, &message).await {
                                break;
                            }
                        }
                        Ok(super::usage::UsageState::Unavailable) => {
                            // Don't send anything for unavailable state;
                            // client keeps its last known good value.
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::debug!("Usage subscriber channel closed");
                            break;
                        }
                        Err(broadcast::error::RecvError::Lagged(count)) => {
                            tracing::warn!("Usage subscriber lagged, missed {} messages", count);
                        }
                    }
                }
            }
        } else {
            // No usage fetcher — session-only mode (backwards-compatible)
            match session_rx.recv().await {
                Ok(update) => {
                    let message = format!(
                        "UPDATE {} {} {}\n",
                        update.session_id, update.status, update.elapsed_seconds
                    );
                    if write_or_disconnect(writer, &message).await {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    tracing::debug!("Subscriber channel closed");
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    tracing::warn!("Subscriber lagged, missed {} messages", count);
                    let lag_message = format!("WARN lagged {}\n", count);
                    if write_or_disconnect(writer, &lag_message).await {
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Writes a message to the client. Returns `true` if the client disconnected.
async fn write_or_disconnect(
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    message: &str,
) -> bool {
    if let Err(e) = writer.write_all(message.as_bytes()).await {
        tracing::debug!("Subscriber disconnected (write failed): {}", e);
        return true;
    }
    if let Err(e) = writer.flush().await {
        tracing::debug!("Subscriber disconnected (flush failed): {}", e);
        return true;
    }
    false
}

/// Handles the RESURRECT command: RESURRECT <session-id>
///
/// Validates that the session exists, is resumable, and its working directory
/// still exists. On success, returns resurrection metadata and removes the
/// session from the closed queue. On failure, returns an error with reason.
///
/// Format: OK <json>\n or ERR <message>\n
pub(super) async fn handle_resurrect_command(args: &[&str], store: &SessionStore) -> String {
    if args.is_empty() {
        return "ERR RESURRECT requires: <session-id>\n".to_string();
    }

    let session_id = args[0];

    let closed = match store.get_closed(session_id).await {
        Some(c) => c,
        None => {
            return format!(
                "ERR SESSION_NOT_FOUND No closed session with ID: {}\n",
                session_id
            );
        }
    };

    if !closed.resumable {
        let reason = closed
            .not_resumable_reason
            .as_deref()
            .unwrap_or("session cannot be resumed");
        return format!("ERR NOT_RESUMABLE {}\n", reason);
    }

    if !closed.working_dir.exists() {
        return format!(
            "ERR WORKING_DIR_MISSING Working directory no longer exists: {}\n",
            closed.working_dir.display()
        );
    }

    let info = serde_json::json!({
        "session_id": closed.session_id,
        "working_dir": closed.working_dir,
        "command": format!("claude --resume {}", closed.session_id),
    });

    store.remove_closed(session_id).await;

    format!("OK {}\n", info)
}

/// Handles the STATUS command: STATUS
///
/// Returns daemon health information as JSON.
/// Format: OK <json>\n
pub(super) async fn handle_status_command(state: &DaemonState) -> String {
    let sessions = state.store.list_all().await;
    let active_count = sessions.iter().filter(|s| !s.closed).count();
    let closed_count = sessions.iter().filter(|s| s.closed).count();

    let health = HealthStatus {
        uptime_seconds: state.start_time.elapsed().as_secs(),
        sessions: SessionCounts {
            active: active_count,
            closed: closed_count,
        },
        connections: state.active_connections.load(Ordering::Relaxed),
        memory_mb: get_memory_usage_mb(),
        socket_path: state.socket_path.clone(),
    };

    let json = serde_json::to_string(&health).expect("failed to serialize HealthStatus");
    format!("OK {}\n", json)
}

/// Handles the DUMP command: DUMP
///
/// Returns a full daemon state snapshot as JSON.
/// Format: OK <json>\n
pub(super) async fn handle_dump_command(state: &DaemonState) -> String {
    let sessions = state.store.list_all().await;
    let active_count = sessions.iter().filter(|s| !s.closed).count();
    let closed_count = sessions.iter().filter(|s| s.closed).count();

    let snapshots: Vec<SessionSnapshot> = sessions
        .iter()
        .map(|s| SessionSnapshot {
            id: s.id.clone(),
            status: s.status.to_string(),
            working_dir: s.working_dir.display().to_string(),
            elapsed_seconds: s.since.elapsed().as_secs(),
            closed: s.closed,
        })
        .collect();

    let dump = DaemonDump {
        uptime_seconds: state.start_time.elapsed().as_secs(),
        socket_path: state.socket_path.clone(),
        sessions: snapshots,
        session_counts: SessionCounts {
            active: active_count,
            closed: closed_count,
        },
        connections: state.active_connections.load(Ordering::Relaxed),
    };

    let json = serde_json::to_string(&dump).expect("failed to serialize DaemonDump");
    format!("OK {}\n", json)
}
