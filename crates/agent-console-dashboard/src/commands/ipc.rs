//! IPC command implementations.
//!
//! Handles client commands that communicate with the daemon via IPC:
//! - `update` - Update session fields (status, priority, working_dir)
//! - `delete` - Delete a session by ID
//! - `status` - Check daemon health
//! - `dump` - Dump full daemon state

use agent_console_dashboard::{
    format_uptime, DaemonDump, HealthStatus, IpcCommand, IpcCommandKind, IpcResponse,
    SessionSnapshot, IPC_VERSION,
};
use std::path::PathBuf;
use std::process::ExitCode;

/// Connects to daemon, sends SET command as JSON to update session fields.
///
/// At least one of status, working_dir, or priority should be provided.
/// If none are provided, prints a warning and returns success.
pub(crate) fn run_update_command(
    socket: &PathBuf,
    session_id: &str,
    status: Option<&str>,
    working_dir: Option<&std::path::Path>,
    priority: Option<u64>,
) -> ExitCode {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;

    // Check if at least one field is provided
    if status.is_none() && working_dir.is_none() && priority.is_none() {
        eprintln!("Warning: no fields to update (specify --status, --working-dir, or --priority)");
        return ExitCode::SUCCESS;
    }

    let stream = match UnixStream::connect(socket) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Error: daemon not running (cannot connect to {:?})", socket);
            return ExitCode::FAILURE;
        }
    };

    let mut writer = stream.try_clone().expect("failed to clone unix stream");
    let mut reader = BufReader::new(stream);

    let wd = working_dir.map(|p| p.display().to_string());

    let cmd = IpcCommand {
        version: IPC_VERSION,
        cmd: IpcCommandKind::Set.to_string(),
        session_id: Some(session_id.to_string()),
        status: status.map(|s| s.to_string()),
        working_dir: wd,
        confirmed: None,
        priority,
    };
    let json = serde_json::to_string(&cmd).expect("failed to serialize SET command");
    let line = format!("{}\n", json);

    if writer.write_all(line.as_bytes()).is_err() || writer.flush().is_err() {
        eprintln!("Error: failed to send SET command");
        return ExitCode::FAILURE;
    }

    let mut response = String::new();
    if reader.read_line(&mut response).is_err() {
        eprintln!("Error: failed to read daemon response");
        return ExitCode::FAILURE;
    }

    match serde_json::from_str::<IpcResponse>(response.trim()) {
        Ok(resp) if resp.ok => ExitCode::SUCCESS,
        Ok(resp) => {
            eprintln!(
                "Error: {}",
                resp.error.unwrap_or_else(|| "unknown error".to_string())
            );
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("Error: failed to parse daemon response: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// Connects to daemon, sends DELETE command to remove a session.
///
/// On success, prints the deleted SessionSnapshot to stdout as JSON.
/// On failure, prints error message to stderr.
pub(crate) fn run_delete_command(socket: &PathBuf, session_id: &str) -> ExitCode {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;

    let stream = match UnixStream::connect(socket) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Error: daemon not running (cannot connect to {:?})", socket);
            return ExitCode::FAILURE;
        }
    };

    let mut writer = stream.try_clone().expect("failed to clone unix stream");
    let mut reader = BufReader::new(stream);

    let cmd = IpcCommand {
        version: IPC_VERSION,
        cmd: IpcCommandKind::Delete.to_string(),
        session_id: Some(session_id.to_string()),
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };
    let json = serde_json::to_string(&cmd).expect("failed to serialize DELETE command");
    let line = format!("{}\n", json);

    if writer.write_all(line.as_bytes()).is_err() || writer.flush().is_err() {
        eprintln!("Error: failed to send DELETE command");
        return ExitCode::FAILURE;
    }

    let mut response = String::new();
    if reader.read_line(&mut response).is_err() {
        eprintln!("Error: failed to read daemon response");
        return ExitCode::FAILURE;
    }

    match serde_json::from_str::<IpcResponse>(response.trim()) {
        Ok(resp) if resp.ok => {
            if let Some(data) = resp.data {
                // Parse as SessionSnapshot and print to stdout
                match serde_json::from_value::<SessionSnapshot>(data.clone()) {
                    Ok(snapshot) => {
                        println!(
                            "{}",
                            serde_json::to_string(&snapshot)
                                .expect("failed to re-serialize SessionSnapshot")
                        );
                        return ExitCode::SUCCESS;
                    }
                    Err(e) => {
                        eprintln!("Error: failed to parse deleted session data: {}", e);
                        return ExitCode::FAILURE;
                    }
                }
            }
            eprintln!("Error: unexpected response - no session data in DELETE response");
            ExitCode::FAILURE
        }
        Ok(resp) => {
            eprintln!(
                "Error: {}",
                resp.error.unwrap_or_else(|| "unknown error".to_string())
            );
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("Error: failed to parse daemon response: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// Connects to the daemon socket, sends STATUS as JSON, and displays health info.
///
/// Returns `ExitCode::SUCCESS` if the daemon is running, `ExitCode::FAILURE` if unreachable.
pub(crate) fn run_status_command(socket: &PathBuf) -> ExitCode {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;

    let stream = match UnixStream::connect(socket) {
        Ok(s) => s,
        Err(_) => {
            println!("Agent Console Daemon");
            println!("  Status:      not running");
            return ExitCode::FAILURE;
        }
    };

    let mut writer = stream.try_clone().expect("failed to clone unix stream");
    let mut reader = BufReader::new(stream);

    let cmd = IpcCommand {
        version: IPC_VERSION,
        cmd: IpcCommandKind::Status.to_string(),
        session_id: None,
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };
    let json = serde_json::to_string(&cmd).expect("failed to serialize STATUS command");
    let line = format!("{}\n", json);

    if writer.write_all(line.as_bytes()).is_err() || writer.flush().is_err() {
        println!("Agent Console Daemon");
        println!("  Status:      not running");
        return ExitCode::FAILURE;
    }

    let mut response = String::new();
    if reader.read_line(&mut response).is_err() {
        println!("Agent Console Daemon");
        println!("  Status:      not running");
        return ExitCode::FAILURE;
    }

    match serde_json::from_str::<IpcResponse>(response.trim()) {
        Ok(resp) if resp.ok => {
            if let Some(data) = resp.data {
                match serde_json::from_value::<HealthStatus>(data) {
                    Ok(health) => {
                        let memory_str = match health.memory_mb {
                            Some(mb) => format!("{:.1} MB", mb),
                            None => "N/A".to_string(),
                        };
                        println!("Agent Console Daemon");
                        println!("  Status:      running");
                        println!("  Uptime:      {}", format_uptime(health.uptime_seconds));
                        println!(
                            "  Sessions:    {} active, {} closed",
                            health.sessions.active, health.sessions.closed
                        );
                        println!("  Connections: {} dashboards", health.connections);
                        println!("  Memory:      {}", memory_str);
                        println!("  Socket:      {}", health.socket_path);
                        return ExitCode::SUCCESS;
                    }
                    Err(e) => {
                        eprintln!("Failed to parse health data: {}", e);
                        return ExitCode::FAILURE;
                    }
                }
            }
            eprintln!("Unexpected response: no data in STATUS response");
            ExitCode::FAILURE
        }
        Ok(resp) => {
            eprintln!(
                "Error: {}",
                resp.error.unwrap_or_else(|| "unknown error".to_string())
            );
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("Failed to parse daemon response: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// Connects to the daemon socket, sends DUMP as JSON, and prints raw JSON.
///
/// Returns `ExitCode::SUCCESS` if the daemon responds, `ExitCode::FAILURE` if unreachable.
pub(crate) fn run_dump_command(socket: &PathBuf) -> ExitCode {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;

    let stream = match UnixStream::connect(socket) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Error: daemon not running (cannot connect to {:?})", socket);
            return ExitCode::FAILURE;
        }
    };

    let mut writer = stream.try_clone().expect("failed to clone unix stream");
    let mut reader = BufReader::new(stream);

    let cmd = IpcCommand {
        version: IPC_VERSION,
        cmd: IpcCommandKind::Dump.to_string(),
        session_id: None,
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };
    let json = serde_json::to_string(&cmd).expect("failed to serialize DUMP command");
    let line = format!("{}\n", json);

    if writer.write_all(line.as_bytes()).is_err() || writer.flush().is_err() {
        eprintln!("Error: failed to send DUMP command");
        return ExitCode::FAILURE;
    }

    let mut response = String::new();
    if reader.read_line(&mut response).is_err() {
        eprintln!("Error: failed to read daemon response");
        return ExitCode::FAILURE;
    }

    match serde_json::from_str::<IpcResponse>(response.trim()) {
        Ok(resp) if resp.ok => {
            if let Some(data) = resp.data {
                // Validate it parses as DaemonDump, then print raw JSON
                match serde_json::from_value::<DaemonDump>(data.clone()) {
                    Ok(_) => {
                        println!(
                            "{}",
                            serde_json::to_string(&data).expect("failed to re-serialize dump data")
                        );
                        return ExitCode::SUCCESS;
                    }
                    Err(e) => {
                        eprintln!("Failed to parse dump data: {}", e);
                        return ExitCode::FAILURE;
                    }
                }
            }
            eprintln!("Unexpected response: no data in DUMP response");
            ExitCode::FAILURE
        }
        Ok(resp) => {
            eprintln!(
                "Error: {}",
                resp.error.unwrap_or_else(|| "unknown error".to_string())
            );
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("Failed to parse daemon response: {}", e);
            ExitCode::FAILURE
        }
    }
}
