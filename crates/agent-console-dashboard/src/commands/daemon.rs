//! Daemon lifecycle command implementations.
//!
//! Handles daemon management commands:
//! - `stop` - Send stop command to running daemon (with confirmation)
//! - `is_daemon_running` - Check if daemon is reachable

use agent_console_dashboard::{IpcCommand, IpcCommandKind, IpcResponse, IPC_VERSION};
use std::process::ExitCode;

/// Checks if daemon is already running by attempting to connect to the socket.
///
/// Returns `true` if the socket exists and accepts connections, `false` otherwise.
///
/// When starting a daemon, this function is used to check for an existing daemon
/// instance. If found, the existing daemon is reused and no new daemon is started.
/// This prevents duplicate daemon processes and preserves the state of the running
/// daemon. See the "Reusing existing daemon... no new daemon started" message in
/// `acd daemon start`.
pub(crate) fn is_daemon_running(socket: &std::path::Path) -> bool {
    use std::os::unix::net::UnixStream;
    UnixStream::connect(socket).is_ok()
}

/// Connects to daemon, sends STOP command, handles confirmation, and triggers shutdown.
///
/// If `force` is true, skips the confirmation prompt and stops the daemon immediately.
pub(crate) fn run_daemon_stop_command(socket: &std::path::Path, force: bool) -> ExitCode {
    use std::io::{self, BufRead, BufReader, Write};
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

    // Send initial STOP command (without confirmation)
    let cmd = IpcCommand {
        version: IPC_VERSION,
        cmd: IpcCommandKind::Stop.to_string(),
        session_id: None,
        status: None,
        working_dir: None,
        confirmed: None,
        priority: None,
    };
    let json = serde_json::to_string(&cmd).expect("failed to serialize STOP command");
    let line = format!("{}\n", json);

    if writer.write_all(line.as_bytes()).is_err() || writer.flush().is_err() {
        eprintln!("Error: failed to send STOP command");
        return ExitCode::FAILURE;
    }

    let mut response = String::new();
    if reader.read_line(&mut response).is_err() {
        eprintln!("Error: failed to read daemon response");
        return ExitCode::FAILURE;
    }

    match serde_json::from_str::<IpcResponse>(response.trim()) {
        Ok(resp) if resp.ok => {
            // Check if this is a confirm_required response
            if let Some(data) = &resp.data {
                if let Some(stop_status) = data.get("stop_status").and_then(|v| v.as_str()) {
                    if stop_status == "confirm_required" {
                        let count = data
                            .get("active_count")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize;

                        let confirmed = if force {
                            // Force mode: skip prompt and auto-confirm
                            true
                        } else {
                            // Interactive mode: prompt user
                            println!("Warning: {} active session(s) are running.", count);
                            println!("Stopping the daemon will disconnect the TUI but Claude Code sessions will continue.");
                            print!("Stop daemon anyway? (y/N): ");
                            io::stdout().flush().expect("failed to flush stdout");

                            let mut input = String::new();
                            if io::stdin().read_line(&mut input).is_err() {
                                eprintln!("Error: failed to read user input");
                                return ExitCode::FAILURE;
                            }

                            input.trim().eq_ignore_ascii_case("y")
                        };

                        if !confirmed {
                            println!("Cancelled.");
                            return ExitCode::SUCCESS;
                        }

                        // Send STOP with confirmation
                        let cmd_confirmed = IpcCommand {
                            version: IPC_VERSION,
                            cmd: IpcCommandKind::Stop.to_string(),
                            session_id: None,
                            status: None,
                            working_dir: None,
                            confirmed: Some(true),
                            priority: None,
                        };
                        let json_confirmed = serde_json::to_string(&cmd_confirmed)
                            .expect("failed to serialize STOP command");
                        let line_confirmed = format!("{}\n", json_confirmed);

                        if writer.write_all(line_confirmed.as_bytes()).is_err()
                            || writer.flush().is_err()
                        {
                            eprintln!("Error: failed to send confirmed STOP command");
                            return ExitCode::FAILURE;
                        }

                        let mut response_confirmed = String::new();
                        if reader.read_line(&mut response_confirmed).is_err() {
                            eprintln!("Error: failed to read daemon response");
                            return ExitCode::FAILURE;
                        }

                        return match serde_json::from_str::<IpcResponse>(response_confirmed.trim())
                        {
                            Ok(resp_confirmed) if resp_confirmed.ok => {
                                println!("Daemon stopped.");
                                ExitCode::SUCCESS
                            }
                            Ok(resp_confirmed) => {
                                eprintln!(
                                    "Error: {}",
                                    resp_confirmed
                                        .error
                                        .unwrap_or_else(|| "unknown error".to_string())
                                );
                                ExitCode::FAILURE
                            }
                            Err(e) => {
                                eprintln!("Error: failed to parse daemon response: {}", e);
                                ExitCode::FAILURE
                            }
                        };
                    } else if stop_status == "ok" {
                        println!("Daemon stopped.");
                        return ExitCode::SUCCESS;
                    }
                }
            }
            // If we get here, response is ok but not a STOP response
            println!("Daemon stopped.");
            ExitCode::SUCCESS
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

/// Opens the config file in the user's editor ($VISUAL or $EDITOR).
///
/// Backs up the config before opening the editor. Returns error if config does not exist.
pub(crate) fn run_config_edit_command(
) -> Result<(), agent_console_dashboard::config::error::ConfigError> {
    use agent_console_dashboard::config::{default, xdg};
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    let config_path = xdg::config_path();

    // Check if config exists
    if !config_path.exists() {
        return Err(
            agent_console_dashboard::config::error::ConfigError::NotFound {
                path: config_path,
                message: "No config found. Run 'acd config init' first.".to_string(),
            },
        );
    }

    // Determine editor from environment
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .map_err(|_| agent_console_dashboard::config::error::ConfigError::EditorNotSet)?;

    // Back up config with tinydate format
    let tinydate = default::generate_tinydate();
    let backup_path = PathBuf::from(format!("{}.bak.{}", config_path.display(), tinydate));

    fs::copy(&config_path, &backup_path).map_err(|e| {
        agent_console_dashboard::config::error::ConfigError::WriteError {
            path: backup_path.clone(),
            source: e,
        }
    })?;

    println!("Config backed up to: {}", backup_path.display());
    println!("Opening {} in editor...", config_path.display());

    // Open editor via the shell so that EDITOR values like `code-insiders --wait`
    // or `vim -u NONE` are word-split correctly.  Direct Command::new() would
    // look for a binary whose name is the entire EDITOR string (no word splitting
    // outside of a shell).
    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("{} \"$1\"", &editor))
        .arg("--") // $0 (argv[0] for the shell)
        .arg(&config_path) // $1 — the file to edit
        .status()
        .map_err(
            |e| agent_console_dashboard::config::error::ConfigError::EditorError {
                editor: editor.clone(),
                source: e,
            },
        )?;

    if !status.success() {
        return Err(
            agent_console_dashboard::config::error::ConfigError::EditorFailed {
                editor,
                code: status.code(),
            },
        );
    }

    Ok(())
}
