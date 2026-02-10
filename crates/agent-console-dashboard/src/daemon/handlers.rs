//! Command handlers for the daemon socket protocol.
//!
//! Each `handle_*` function processes a single JSON IPC command received from
//! a client connection and returns a JSON Lines response string (or streams
//! data for SUB).

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::io::AsyncWriteExt;
use tokio::sync::broadcast;

use crate::daemon::store::SessionStore;
use crate::daemon::usage::UsageFetcher;
use crate::{
    get_memory_usage_mb, AgentType, DaemonDump, HealthStatus, IpcCommand, IpcNotification,
    IpcResponse, SessionCounts, SessionSnapshot, Status,
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

/// Handles the SET command.
///
/// Expects `cmd.session_id` and `cmd.status`. Optional `cmd.working_dir`.
/// Creates a new session if it doesn't exist, or updates the status if it does.
pub(super) async fn handle_set_command(cmd: &IpcCommand, store: &SessionStore) -> String {
    let session_id = match &cmd.session_id {
        Some(id) => id,
        None => return IpcResponse::error("SET requires session_id").to_json_line(),
    };

    let status_str = match &cmd.status {
        Some(s) => s,
        None => return IpcResponse::error("SET requires status").to_json_line(),
    };

    let working_dir = cmd
        .working_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_default();

    let status: Status = match status_str.parse() {
        Ok(s) => s,
        Err(_) => {
            return IpcResponse::error(format!(
                "invalid status: {} (expected: working, attention, question, closed)",
                status_str
            ))
            .to_json_line();
        }
    };

    let session = store
        .get_or_create_session(
            session_id.clone(),
            AgentType::ClaudeCode,
            working_dir,
            None,
            status,
        )
        .await;

    let info = SessionSnapshot::from(&session);
    IpcResponse::success(Some(
        serde_json::to_value(&info).expect("failed to serialize SessionSnapshot"),
    ))
    .to_json_line()
}

/// Handles the RM command.
///
/// Expects `cmd.session_id`. Closes the session (marks as closed, doesn't
/// remove from store).
pub(super) async fn handle_rm_command(cmd: &IpcCommand, store: &SessionStore) -> String {
    let session_id = match &cmd.session_id {
        Some(id) => id,
        None => return IpcResponse::error("RM requires session_id").to_json_line(),
    };

    match store.close_session(session_id).await {
        Some(session) => {
            let info = SessionSnapshot::from(&session);
            IpcResponse::success(Some(
                serde_json::to_value(&info).expect("failed to serialize SessionSnapshot"),
            ))
            .to_json_line()
        }
        None => IpcResponse::error(format!("session not found: {}", session_id)).to_json_line(),
    }
}

/// Handles the LIST command.
///
/// Returns all sessions as an array of `SessionSnapshot` objects.
pub(super) async fn handle_list_command(store: &SessionStore) -> String {
    let sessions = store.list_all().await;
    let infos: Vec<SessionSnapshot> = sessions.iter().map(SessionSnapshot::from).collect();

    IpcResponse::success(Some(
        serde_json::to_value(&infos).expect("failed to serialize session list"),
    ))
    .to_json_line()
}

/// Handles the GET command.
///
/// Expects `cmd.session_id`. Returns a single `SessionSnapshot`.
pub(super) async fn handle_get_command(cmd: &IpcCommand, store: &SessionStore) -> String {
    let session_id = match &cmd.session_id {
        Some(id) => id,
        None => return IpcResponse::error("GET requires session_id").to_json_line(),
    };

    match store.get(session_id).await {
        Some(session) => {
            let info = SessionSnapshot::from(&session);
            IpcResponse::success(Some(
                serde_json::to_value(&info).expect("failed to serialize SessionSnapshot"),
            ))
            .to_json_line()
        }
        None => IpcResponse::error(format!("session not found: {}", session_id)).to_json_line(),
    }
}

/// Handles the SUB command.
///
/// Subscribes to session updates and usage updates, sending JSON notifications.
///
/// Wire format (JSON Lines):
/// - Session updates: `IpcNotification` with type "update"
/// - Usage updates: `IpcNotification` with type "usage"
/// - Lag warnings: `IpcNotification` with type "warn"
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
    let ok_msg = IpcResponse::success(Some(serde_json::json!("subscribed")));
    writer.write_all(ok_msg.to_json_line().as_bytes()).await?;
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
            let notification = IpcNotification::usage_update(&data);
            if write_or_disconnect(writer, &notification.to_json_line()).await {
                return Ok(());
            }
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
                            // Look up the full session to send complete SessionSnapshot
                            let notification = if let Some(session) = store.get(&update.session_id).await {
                                let info = SessionSnapshot::from(&session);
                                IpcNotification::session_update(info)
                            } else {
                                // Session might have been removed; send minimal info
                                let info = SessionSnapshot {
                                    session_id: update.session_id.clone(),
                                    agent_type: "claudecode".to_string(),
                                    status: update.status.to_string(),
                                    working_dir: None,
                                    elapsed_seconds: update.elapsed_seconds,
                                    idle_seconds: 0,
                                    history: vec![],
                                    closed: update.status == Status::Closed,

                                };
                                IpcNotification::session_update(info)
                            };
                            if write_or_disconnect(writer, &notification.to_json_line()).await {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::debug!("Session subscriber channel closed");
                            break;
                        }
                        Err(broadcast::error::RecvError::Lagged(count)) => {
                            tracing::warn!("Session subscriber lagged, missed {} messages", count);
                            let notification = IpcNotification::warn(format!("lagged {}", count));
                            if write_or_disconnect(writer, &notification.to_json_line()).await {
                                break;
                            }
                        }
                    }
                }
                result = usage.recv() => {
                    match result {
                        Ok(super::usage::UsageState::Available(data)) => {
                            let notification = IpcNotification::usage_update(&data);
                            if write_or_disconnect(writer, &notification.to_json_line()).await {
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
            // No usage fetcher -- session-only mode (backwards-compatible)
            match session_rx.recv().await {
                Ok(update) => {
                    let notification = if let Some(session) = store.get(&update.session_id).await {
                        let info = SessionSnapshot::from(&session);
                        IpcNotification::session_update(info)
                    } else {
                        let info = SessionSnapshot {
                            session_id: update.session_id.clone(),
                            agent_type: "claudecode".to_string(),
                            status: update.status.to_string(),
                            working_dir: None,
                            elapsed_seconds: update.elapsed_seconds,
                            idle_seconds: 0,
                            history: vec![],
                            closed: update.status == Status::Closed,
                        };
                        IpcNotification::session_update(info)
                    };
                    if write_or_disconnect(writer, &notification.to_json_line()).await {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    tracing::debug!("Subscriber channel closed");
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    tracing::warn!("Subscriber lagged, missed {} messages", count);
                    let notification = IpcNotification::warn(format!("lagged {}", count));
                    if write_or_disconnect(writer, &notification.to_json_line()).await {
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Writes a message to the client. Returns `true` if the client disconnected.
async fn write_or_disconnect(writer: &mut tokio::net::unix::OwnedWriteHalf, message: &str) -> bool {
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

/// Handles the RESURRECT command.
///
/// Expects `cmd.session_id`. Validates that the session exists, is resumable,
/// and its working directory still exists. On success, returns resurrection
/// metadata and removes the session from the closed queue.
pub(super) async fn handle_resurrect_command(cmd: &IpcCommand, store: &SessionStore) -> String {
    let session_id = match &cmd.session_id {
        Some(id) => id,
        None => return IpcResponse::error("RESURRECT requires session_id").to_json_line(),
    };

    let closed = match store.get_closed(session_id).await {
        Some(c) => c,
        None => {
            return IpcResponse::error(format!(
                "SESSION_NOT_FOUND No closed session with ID: {}",
                session_id
            ))
            .to_json_line();
        }
    };

    if !closed.resumable {
        let reason = closed
            .not_resumable_reason
            .as_deref()
            .unwrap_or("session cannot be resumed");
        return IpcResponse::error(format!("NOT_RESUMABLE {}", reason)).to_json_line();
    }

    if !closed.working_dir.exists() {
        return IpcResponse::error(format!(
            "WORKING_DIR_MISSING Working directory no longer exists: {}",
            closed.working_dir.display()
        ))
        .to_json_line();
    }

    let info = serde_json::json!({
        "session_id": closed.session_id,
        "working_dir": closed.working_dir,
        "command": format!("claude --resume {}", closed.session_id),
    });

    store.remove_closed(session_id).await;

    IpcResponse::success(Some(info)).to_json_line()
}

/// Handles the STATUS command.
///
/// Returns daemon health information as JSON.
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

    IpcResponse::success(Some(
        serde_json::to_value(&health).expect("failed to serialize HealthStatus"),
    ))
    .to_json_line()
}

/// Handles the DUMP command.
///
/// Returns a full daemon state snapshot as JSON.
pub(super) async fn handle_dump_command(state: &DaemonState) -> String {
    let sessions = state.store.list_all().await;
    let active_count = sessions.iter().filter(|s| !s.closed).count();
    let closed_count = sessions.iter().filter(|s| s.closed).count();

    let snapshots: Vec<crate::DumpSession> = sessions
        .iter()
        .map(|s| crate::DumpSession {
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

    IpcResponse::success(Some(
        serde_json::to_value(&dump).expect("failed to serialize DaemonDump"),
    ))
    .to_json_line()
}
