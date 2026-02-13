//! Agent Console Dashboard - CLI entry point
//!
//! This binary provides the command-line interface for the Agent Console daemon.
//! It supports running in foreground or daemonized mode with configurable socket paths.

use agent_console_dashboard::{
    client::connect_with_lazy_start, daemon::run_daemon, format_uptime, tui::app::App,
    DaemonConfig, DaemonDump, HealthStatus, IpcCommand, IpcResponse, Status, IPC_VERSION,
};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

/// Agent Console Dashboard daemon
#[derive(Parser)]
#[command(name = "agent-console-dashboard")]
#[command(version, about = "Agent Console Dashboard daemon")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Available subcommands for the agent-console CLI
#[derive(Subcommand)]
enum Commands {
    /// Launch the terminal user interface
    Tui {
        /// Socket path for IPC communication
        #[arg(long, default_value = "/tmp/agent-console-dashboard.sock")]
        socket: PathBuf,
    },

    /// Check daemon health status
    Status {
        /// Socket path for IPC communication
        #[arg(long, default_value = "/tmp/agent-console-dashboard.sock")]
        socket: PathBuf,
    },

    /// Dump full daemon state as JSON
    Dump {
        /// Socket path for IPC communication
        #[arg(long, default_value = "/tmp/agent-console-dashboard.sock")]
        socket: PathBuf,
        /// Output format (only json supported in v0)
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Manage configuration file
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Resurrect a previously closed session
    Resurrect {
        /// Session ID to resurrect
        session_id: String,
        /// Socket path for IPC communication
        #[arg(long, default_value = "/tmp/agent-console-dashboard.sock")]
        socket: PathBuf,
        /// Print command without explanation (for scripting)
        #[arg(long)]
        quiet: bool,
    },

    /// Set session status (used by hooks)
    Set {
        /// Session ID
        session_id: String,
        /// Status (working, attention, question, closed)
        status: String,
        /// Working directory
        #[arg(long)]
        working_dir: Option<PathBuf>,
        /// Socket path for IPC communication
        #[arg(long, default_value = "/tmp/agent-console-dashboard.sock")]
        socket: PathBuf,
    },

    /// Daemon management
    Daemon {
        #[command(subcommand)]
        command: DaemonCommands,
    },

    /// Handle Claude Code hook events (reads JSON from stdin)
    ClaudeHook {
        /// Status to set: working, attention, question, closed
        status: Status,
        /// Daemon socket path
        #[arg(long, default_value = "/tmp/agent-console-dashboard.sock")]
        socket: PathBuf,
    },

    /// Install ACD hooks into Claude Code settings (~/.claude/settings.json)
    Install,

    /// Remove ACD hooks from Claude Code settings
    Uninstall,
}

/// Daemon management subcommands
#[derive(Subcommand)]
enum DaemonCommands {
    /// Start the daemon
    Start {
        /// Socket path for IPC communication
        #[arg(long, default_value = "/tmp/agent-console-dashboard.sock")]
        socket: PathBuf,
        /// Run daemon in background (detach from terminal)
        #[arg(short, long)]
        detach: bool,
    },
    /// Stop the running daemon
    Stop {
        /// Socket path for IPC communication
        #[arg(long, default_value = "/tmp/agent-console-dashboard.sock")]
        socket: PathBuf,
        /// Stop without confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

/// Actions for the `config` subcommand.
#[derive(Subcommand)]
enum ConfigAction {
    /// Create default configuration file
    Init {
        /// Overwrite existing configuration (creates backup)
        #[arg(long)]
        force: bool,
    },
    /// Show configuration file path
    Path,
    /// Validate configuration file
    Validate,
    /// Display current effective configuration
    Show,
}

fn main() -> ExitCode {
    // Parse CLI arguments BEFORE any fork/runtime operations
    // This ensures errors are shown to the user in the terminal
    let cli = Cli::parse();

    match cli.command {
        Commands::Tui { socket } => {
            let rt =
                tokio::runtime::Runtime::new().expect("failed to create tokio runtime for TUI");
            if let Err(e) = rt.block_on(async {
                let mut app = App::new(socket);
                // Wire double-click hook from config if available
                if let Ok(config) =
                    agent_console_dashboard::config::loader::ConfigLoader::load_default()
                {
                    if !config.tui.double_click_hook.is_empty() {
                        app.double_click_hook = Some(config.tui.double_click_hook);
                    }
                }
                app.run().await
            }) {
                eprintln!("TUI error: {}", e);
                return ExitCode::FAILURE;
            }
        }
        Commands::Status { socket } => {
            return run_status_command(&socket);
        }
        Commands::Dump { socket, format } => {
            if format != "json" {
                eprintln!(
                    "Error: format '{}' not yet implemented, only 'json' is supported",
                    format
                );
                return ExitCode::FAILURE;
            }
            return run_dump_command(&socket);
        }
        Commands::Resurrect {
            session_id,
            socket,
            quiet,
        } => {
            return run_resurrect_command(&socket, &session_id, quiet);
        }
        Commands::Config { action } => {
            use agent_console_dashboard::config::{default, loader::ConfigLoader, xdg};
            let result = match action {
                ConfigAction::Init { force } => match default::create_default_config(force) {
                    Ok(_path) => {
                        // Output is handled by create_default_config
                        Ok(())
                    }
                    Err(e) => Err(e),
                },
                ConfigAction::Path => {
                    println!("{}", xdg::config_path().display());
                    Ok(())
                }
                ConfigAction::Validate => match ConfigLoader::load_default() {
                    Ok(config) => {
                        println!("Configuration is valid");
                        println!("{config:#?}");
                        Ok(())
                    }
                    Err(e) => Err(e),
                },
                ConfigAction::Show => match ConfigLoader::load_default() {
                    Ok(config) => {
                        let config_path = xdg::config_path();
                        if config_path.exists() {
                            println!("# Configuration loaded from: {}", config_path.display());
                        } else {
                            println!("# No config file found (showing built-in defaults)");
                            println!(
                                "# Run 'acd config init' to create: {}",
                                config_path.display()
                            );
                        }
                        println!();
                        match toml::to_string_pretty(&config) {
                            Ok(toml_str) => {
                                println!("{}", toml_str);
                                Ok(())
                            }
                            Err(e) => Err(agent_console_dashboard::config::error::ConfigError::SerializeError {
                                message: format!("failed to serialize config: {}", e),
                            }),
                        }
                    }
                    Err(e) => Err(e),
                },
            };
            if let Err(e) = result {
                eprintln!("Config error: {e}");
                return ExitCode::FAILURE;
            }
        }
        Commands::Set {
            session_id,
            status,
            working_dir,
            socket,
        } => {
            return run_set_command(&socket, &session_id, &status, working_dir.as_deref());
        }
        Commands::Daemon { command } => match command {
            DaemonCommands::Start { socket, detach } => {
                // Check if daemon is already running
                if is_daemon_running(&socket) {
                    println!(
                        "Reusing existing daemon on {} (no new daemon started)",
                        socket.display()
                    );
                    return ExitCode::SUCCESS;
                }

                // Create DaemonConfig from CLI args
                let config = DaemonConfig::new(socket, detach);

                // Run the daemon - this will:
                // 1. Call daemonize_process() if --detach flag set
                // 2. Start Tokio runtime AFTER daemonization
                // 3. Wait for shutdown signal (SIGINT/SIGTERM)
                if let Err(e) = run_daemon(config) {
                    eprintln!("Error: {}", e);
                    return ExitCode::FAILURE;
                }
            }
            DaemonCommands::Stop { socket, force } => {
                return run_daemon_stop_command(&socket, force);
            }
        },
        Commands::ClaudeHook { status, socket } => {
            // Parse stdin synchronously before creating async runtime
            let input: HookInput = match serde_json::from_reader(std::io::stdin()) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("acd claude-hook: failed to parse JSON from stdin: {}", e);
                    return ExitCode::from(2);
                }
            };

            let rt =
                tokio::runtime::Runtime::new().expect("failed to create tokio runtime for hook");
            return rt.block_on(run_claude_hook_async(&socket, status, &input));
        }
        Commands::Install => {
            return run_install_command();
        }
        Commands::Uninstall => {
            return run_uninstall_command();
        }
    }

    ExitCode::SUCCESS
}

/// Checks if daemon is already running by attempting to connect to the socket.
///
/// Returns `true` if the socket exists and accepts connections, `false` otherwise.
fn is_daemon_running(socket: &std::path::Path) -> bool {
    use std::os::unix::net::UnixStream;
    UnixStream::connect(socket).is_ok()
}

/// Connects to daemon, sends STOP command, handles confirmation, and triggers shutdown.
///
/// If `force` is true, skips the confirmation prompt and stops the daemon immediately.
fn run_daemon_stop_command(socket: &std::path::Path, force: bool) -> ExitCode {
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
        cmd: "STOP".to_string(),
        session_id: None,
        status: None,
        working_dir: None,
        confirmed: None,
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
                            cmd: "STOP".to_string(),
                            session_id: None,
                            status: None,
                            working_dir: None,
                            confirmed: Some(true),
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

/// Connects to daemon, sends SET command as JSON to create/update a session.
fn run_set_command(
    socket: &PathBuf,
    session_id: &str,
    status: &str,
    working_dir: Option<&std::path::Path>,
) -> ExitCode {
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

    let wd = working_dir
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| {
            std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".to_string())
        });

    let cmd = IpcCommand {
        version: IPC_VERSION,
        cmd: "SET".to_string(),
        session_id: Some(session_id.to_string()),
        status: Some(status.to_string()),
        working_dir: Some(wd),
        confirmed: None,
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

/// JSON payload from Claude Code hook stdin.
///
/// Only fields we need are declared; unknown fields are silently ignored
/// so future Claude Code versions don't break us.
#[derive(serde::Deserialize)]
struct HookInput {
    session_id: String,
    cwd: String,
}

/// Validates HookInput fields. Returns warnings for invalid fields.
/// Does not reject input — Claude Code should not be blocked by validation.
fn validate_hook_input(input: &HookInput) -> Vec<String> {
    let mut warnings = Vec::new();

    // session_id: 36 chars, hex + dashes only
    // TODO(acd-rhr): Consider full UUID v4 validation
    if input.session_id.len() != 36 {
        warnings.push(format!(
            "session_id length is {} (expected 36): {}",
            input.session_id.len(),
            input.session_id
        ));
    } else if !input
        .session_id
        .chars()
        .all(|c| c.is_ascii_hexdigit() || c == '-')
    {
        warnings.push(format!(
            "session_id contains invalid characters: {}",
            input.session_id
        ));
    }

    // cwd: non-empty absolute path
    // TODO(acd-8vx): Consider validating path exists
    if input.cwd.is_empty() {
        warnings.push("cwd is empty".to_string());
    } else if !input.cwd.starts_with('/') {
        warnings.push(format!("cwd is not an absolute path: {}", input.cwd));
    }

    warnings
}

/// Connects to daemon via lazy-start (spawning if needed), sends SET command as JSON.
///
/// Exit codes per Claude Code hook spec:
/// - 0: success (outputs `{"continue": true}` on stdout)
///
/// This function never returns a non-zero exit code after stdin parsing
/// succeeds -- hook failures are reported via systemMessage to avoid blocking
/// Claude Code.
async fn run_claude_hook_async(
    socket: &std::path::Path,
    status: Status,
    input: &HookInput,
) -> ExitCode {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let warnings = validate_hook_input(input);
    for w in &warnings {
        eprintln!("acd claude-hook: warning: {}", w);
    }

    let client = match connect_with_lazy_start(socket).await {
        Ok(c) => c,
        Err(e) => {
            let json = serde_json::json!({
                "continue": true,
                "systemMessage": format!(
                    "acd daemon not reachable ({}), session {} not tracked",
                    e, input.session_id
                ),
            });
            println!("{}", json);
            return ExitCode::SUCCESS;
        }
    };

    let stream = client.into_stream();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let cmd = IpcCommand {
        version: IPC_VERSION,
        cmd: "SET".to_string(),
        session_id: Some(input.session_id.clone()),
        status: Some(status.to_string()),
        working_dir: Some(input.cwd.clone()),
        confirmed: None,
    };
    let cmd_json = serde_json::to_string(&cmd).expect("failed to serialize SET command");
    let cmd_line = format!("{}\n", cmd_json);

    if writer.write_all(cmd_line.as_bytes()).await.is_err() || writer.flush().await.is_err() {
        let json = serde_json::json!({
            "continue": true,
            "systemMessage": format!(
                "acd daemon: failed to send command, session {} not tracked",
                input.session_id
            ),
        });
        println!("{}", json);
        return ExitCode::SUCCESS;
    }

    let mut line = String::new();
    if reader.read_line(&mut line).await.is_err() {
        let json = serde_json::json!({
            "continue": true,
            "systemMessage": format!(
                "acd daemon: no response, session {} not tracked",
                input.session_id
            ),
        });
        println!("{}", json);
        return ExitCode::SUCCESS;
    }

    match serde_json::from_str::<IpcResponse>(line.trim()) {
        Ok(resp) if resp.ok => {
            println!(r#"{{"continue": true}}"#);
        }
        Ok(resp) => {
            let err = resp.error.unwrap_or_else(|| "unknown error".to_string());
            let json = serde_json::json!({
                "continue": true,
                "systemMessage": format!(
                    "acd daemon error: {}, session {} not tracked",
                    err, input.session_id
                ),
            });
            println!("{}", json);
        }
        Err(_) => {
            let json = serde_json::json!({
                "continue": true,
                "systemMessage": format!(
                    "acd daemon: invalid response, session {} not tracked",
                    input.session_id
                ),
            });
            println!("{}", json);
        }
    }
    ExitCode::SUCCESS
}

/// Connects to the daemon socket, sends STATUS as JSON, and displays health info.
///
/// Returns `ExitCode::SUCCESS` if the daemon is running, `ExitCode::FAILURE` if unreachable.
fn run_status_command(socket: &PathBuf) -> ExitCode {
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
        cmd: "STATUS".to_string(),
        session_id: None,
        status: None,
        working_dir: None,
        confirmed: None,
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
fn run_dump_command(socket: &PathBuf) -> ExitCode {
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
        cmd: "DUMP".to_string(),
        session_id: None,
        status: None,
        working_dir: None,
        confirmed: None,
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

/// Connects to daemon, sends RESURRECT as JSON, and displays resurrection metadata.
fn run_resurrect_command(socket: &PathBuf, session_id: &str, quiet: bool) -> ExitCode {
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
        cmd: "RESURRECT".to_string(),
        session_id: Some(session_id.to_string()),
        status: None,
        working_dir: None,
        confirmed: None,
    };
    let json = serde_json::to_string(&cmd).expect("failed to serialize RESURRECT command");
    let line = format!("{}\n", json);

    if writer.write_all(line.as_bytes()).is_err() || writer.flush().is_err() {
        eprintln!("Error: failed to send RESURRECT command");
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
                let sid = data["session_id"].as_str().unwrap_or(session_id);
                let wd = data["working_dir"].as_str().unwrap_or("<unknown>");
                let resume_cmd = data["command"].as_str().unwrap_or("claude --resume");
                if quiet {
                    println!("cd {} && {}", wd, resume_cmd);
                } else {
                    println!("To resurrect session {}:", sid);
                    println!("  cd {}", wd);
                    println!("  {}", resume_cmd);
                }
                return ExitCode::SUCCESS;
            }
            eprintln!("Unexpected response: no data in RESURRECT response");
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

/// Returns the complete list of ACD hooks to install.
///
/// Each entry: (event, command, timeout, matcher).
/// This is the single source of truth for which hooks ACD registers.
fn acd_hook_definitions() -> Vec<(claude_hooks::HookEvent, &'static str, Option<String>)> {
    use claude_hooks::HookEvent;
    vec![
        (HookEvent::SessionStart, "acd claude-hook attention", None),
        (HookEvent::UserPromptSubmit, "acd claude-hook working", None),
        (HookEvent::Stop, "acd claude-hook attention", None),
        (HookEvent::SessionEnd, "acd claude-hook closed", None),
        (
            HookEvent::Notification,
            "acd claude-hook question",
            Some("elicitation_dialog".to_string()),
        ),
        (
            HookEvent::Notification,
            "acd claude-hook attention",
            Some("permission_prompt".to_string()),
        ),
        // PreToolUse bridges the gap when Claude resumes after permission_prompt
        // or elicitation_dialog. Without it, status stays "attention" while
        // Claude is actively working.
        (HookEvent::PreToolUse, "acd claude-hook working", None),
        // Experiment (acd-ws6): PostToolUse removed to test if PreToolUse alone
        // provides accurate status transitions. Restore when experiment concludes.
        // (HookEvent::PostToolUse, "acd claude-hook working", None),
    ]
}

/// Check if `acd` binary is reachable in PATH.
fn acd_in_path() -> bool {
    std::process::Command::new("which")
        .arg("acd")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Ensure ~/.claude/settings.json exists (create with `{}` if missing).
fn ensure_settings_file() -> std::result::Result<(), String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    let claude_dir = std::path::Path::new(&home).join(".claude");
    let settings_path = claude_dir.join("settings.json");

    if !settings_path.exists() {
        std::fs::create_dir_all(&claude_dir)
            .map_err(|e| format!("failed to create ~/.claude/: {}", e))?;
        std::fs::write(&settings_path, "{}\n")
            .map_err(|e| format!("failed to create settings.json: {}", e))?;
        println!("  Created ~/.claude/settings.json");
    }
    Ok(())
}

/// Install all ACD hooks into ~/.claude/settings.json.
fn run_install_command() -> ExitCode {
    // 1. Check PATH
    if !acd_in_path() {
        eprintln!("Warning: 'acd' not found in PATH");
        eprintln!("  Hooks will fail silently until acd is in PATH.");
        eprintln!("  Fix: cargo install --path crates/agent-console-dashboard");
        eprintln!();
    }

    // 2. Ensure settings.json exists
    if let Err(e) = ensure_settings_file() {
        eprintln!("Error: {}", e);
        return ExitCode::FAILURE;
    }

    // 3. Install each hook
    let definitions = acd_hook_definitions();
    let mut installed = 0u32;
    let mut skipped = 0u32;
    let mut errors = Vec::new();

    for (event, command, matcher) in &definitions {
        let handler = claude_hooks::HookHandler {
            r#type: "command".to_string(),
            command: command.to_string(),
            timeout: Some(10),
            r#async: None,
            status_message: None,
        };

        match claude_hooks::install(*event, handler, matcher.clone(), "acd") {
            Ok(()) => {
                installed += 1;
                let matcher_str = matcher
                    .as_ref()
                    .map(|m| format!(" ({})", m))
                    .unwrap_or_default();
                println!("  Installed: {:?}{} -> {}", event, matcher_str, command);
            }
            Err(claude_hooks::Error::Hook(claude_hooks::HookError::AlreadyExists { .. })) => {
                skipped += 1;
            }
            Err(e) => {
                errors.push(format!("{:?} -> {}: {}", event, command, e));
            }
        }
    }

    // 4. Summary
    println!();
    println!(
        "Hooks: {} installed, {} already present, {} errors",
        installed,
        skipped,
        errors.len()
    );

    if !errors.is_empty() {
        eprintln!();
        for err in &errors {
            eprintln!("  Error: {}", err);
        }
        return ExitCode::FAILURE;
    }

    if installed > 0 {
        println!();
        println!("You may need to restart Claude Code for hooks to take effect.");
    }

    ExitCode::SUCCESS
}

/// Remove all ACD-managed hooks from ~/.claude/settings.json.
fn run_uninstall_command() -> ExitCode {
    let definitions = acd_hook_definitions();
    let mut removed = 0u32;
    let mut skipped = 0u32;
    let mut errors = Vec::new();

    for (event, command, _matcher) in &definitions {
        match claude_hooks::uninstall(*event, command) {
            Ok(()) => {
                removed += 1;
                println!("  Removed: {:?} -> {}", event, command);
            }
            Err(claude_hooks::Error::Hook(claude_hooks::HookError::NotManaged { .. })) => {
                skipped += 1;
            }
            Err(e) => {
                errors.push(format!("{:?} -> {}: {}", event, command, e));
            }
        }
    }

    println!();
    println!(
        "Hooks: {} removed, {} not managed, {} errors",
        removed,
        skipped,
        errors.len()
    );

    if !errors.is_empty() {
        eprintln!();
        for err in &errors {
            eprintln!("  Error: {}", err);
        }
        return ExitCode::FAILURE;
    }

    if removed > 0 {
        println!();
        println!("You may need to restart Claude Code for changes to take effect.");
    }

    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        // Verify the CLI configuration is valid
        Cli::command().debug_assert();
    }

    #[test]
    fn test_daemon_without_subcommand_fails() {
        // Verify bare daemon command requires a subcommand
        let result = Cli::try_parse_from(["agent-console-dashboard", "daemon"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_socket_path() {
        // Verify default socket path is /tmp/agent-console-dashboard.sock
        let cli = Cli::try_parse_from(["agent-console-dashboard", "daemon", "start"]).unwrap();
        match cli.command {
            Commands::Daemon {
                command: DaemonCommands::Start { socket, .. },
            } => {
                assert_eq!(socket, PathBuf::from("/tmp/agent-console-dashboard.sock"));
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn test_custom_socket_path() {
        // Verify custom socket path can be specified
        let cli = Cli::try_parse_from([
            "agent-console-dashboard",
            "daemon",
            "start",
            "--socket",
            "/custom/path.sock",
        ])
        .unwrap();
        match cli.command {
            Commands::Daemon {
                command: DaemonCommands::Start { socket, .. },
            } => {
                assert_eq!(socket, PathBuf::from("/custom/path.sock"));
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn test_detach_flag_default_false() {
        // Verify detach flag defaults to false
        let cli = Cli::try_parse_from(["agent-console-dashboard", "daemon", "start"]).unwrap();
        match cli.command {
            Commands::Daemon {
                command: DaemonCommands::Start { detach, .. },
            } => {
                assert!(!detach);
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn test_detach_flag_true() {
        // Verify detach flag can be set to true
        let cli = Cli::try_parse_from(["agent-console-dashboard", "daemon", "start", "--detach"])
            .unwrap();
        match cli.command {
            Commands::Daemon {
                command: DaemonCommands::Start { detach, .. },
            } => {
                assert!(detach);
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn test_daemon_start_help_contains_expected_options() {
        // Verify that daemon start subcommand help contains --detach and --socket
        let cmd = Cli::command();
        let daemon_cmd = cmd
            .get_subcommands()
            .find(|sc| sc.get_name() == "daemon")
            .expect("daemon subcommand should exist");
        let start_cmd = daemon_cmd
            .get_subcommands()
            .find(|sc| sc.get_name() == "start")
            .expect("daemon start subcommand should exist");

        // Check that --detach option exists
        let detach_arg = start_cmd
            .get_arguments()
            .find(|arg| arg.get_id() == "detach");
        assert!(detach_arg.is_some(), "--detach flag should exist");

        // Check that --socket option exists
        let socket_arg = start_cmd
            .get_arguments()
            .find(|arg| arg.get_id() == "socket");
        assert!(socket_arg.is_some(), "--socket flag should exist");
    }

    #[test]
    fn test_combined_flags() {
        // Verify both flags can be used together
        let cli = Cli::try_parse_from([
            "agent-console-dashboard",
            "daemon",
            "start",
            "--detach",
            "--socket",
            "/var/run/my-daemon.sock",
        ])
        .unwrap();
        match cli.command {
            Commands::Daemon {
                command: DaemonCommands::Start { detach, socket },
            } => {
                assert!(detach);
                assert_eq!(socket, PathBuf::from("/var/run/my-daemon.sock"));
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn test_flag_order_independence() {
        // Verify flags can be specified in any order (--socket before --detach)
        let cli = Cli::try_parse_from([
            "agent-console-dashboard",
            "daemon",
            "start",
            "--socket",
            "/custom/path.sock",
            "--detach",
        ])
        .unwrap();
        match cli.command {
            Commands::Daemon {
                command: DaemonCommands::Start { detach, socket },
            } => {
                assert!(detach);
                assert_eq!(socket, PathBuf::from("/custom/path.sock"));
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn test_unknown_subcommand_fails() {
        // Verify unknown subcommand fails to parse
        let result = Cli::try_parse_from(["agent-console-dashboard", "unknown"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_subcommand_fails() {
        // Verify missing subcommand fails to parse
        let result = Cli::try_parse_from(["agent-console"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_socket_requires_value() {
        // Verify --socket flag requires a value
        let result =
            Cli::try_parse_from(["agent-console-dashboard", "daemon", "start", "--socket"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_socket_path_with_spaces() {
        // Verify socket path with spaces works correctly
        let cli = Cli::try_parse_from([
            "agent-console-dashboard",
            "daemon",
            "start",
            "--socket",
            "/path/with spaces/socket.sock",
        ])
        .unwrap();
        match cli.command {
            Commands::Daemon {
                command: DaemonCommands::Start { socket, .. },
            } => {
                assert_eq!(socket, PathBuf::from("/path/with spaces/socket.sock"));
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn test_socket_relative_path() {
        // Verify relative socket path is accepted
        let cli = Cli::try_parse_from([
            "agent-console-dashboard",
            "daemon",
            "start",
            "--socket",
            "./local.sock",
        ])
        .unwrap();
        match cli.command {
            Commands::Daemon {
                command: DaemonCommands::Start { socket, .. },
            } => {
                assert_eq!(socket, PathBuf::from("./local.sock"));
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn test_unknown_flag_fails() {
        // Verify unknown flag fails to parse
        let result = Cli::try_parse_from(["agent-console-dashboard", "daemon", "--unknown-flag"]);
        assert!(result.is_err());
    }

    // -- ClaudeHook subcommand ------------------------------------------------

    #[test]
    fn test_claude_hook_working_parses() {
        let cli = Cli::try_parse_from(["agent-console-dashboard", "claude-hook", "working"])
            .expect("claude-hook working should parse");
        match cli.command {
            Commands::ClaudeHook { status, socket } => {
                assert_eq!(status, agent_console_dashboard::Status::Working);
                assert_eq!(socket, PathBuf::from("/tmp/agent-console-dashboard.sock"));
            }
            _ => panic!("expected ClaudeHook command"),
        }
    }

    #[test]
    fn test_claude_hook_attention_parses() {
        let cli = Cli::try_parse_from(["agent-console-dashboard", "claude-hook", "attention"])
            .expect("claude-hook attention should parse");
        match cli.command {
            Commands::ClaudeHook { status, .. } => {
                assert_eq!(status, agent_console_dashboard::Status::Attention);
            }
            _ => panic!("expected ClaudeHook command"),
        }
    }

    #[test]
    fn test_claude_hook_custom_socket() {
        let cli = Cli::try_parse_from([
            "agent-console-dashboard",
            "claude-hook",
            "working",
            "--socket",
            "/custom/path.sock",
        ])
        .expect("claude-hook with custom socket should parse");
        match cli.command {
            Commands::ClaudeHook { socket, .. } => {
                assert_eq!(socket, PathBuf::from("/custom/path.sock"));
            }
            _ => panic!("expected ClaudeHook command"),
        }
    }

    #[test]
    fn test_claude_hook_requires_status() {
        let result = Cli::try_parse_from(["agent-console-dashboard", "claude-hook"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_claude_hook_invalid_status_fails() {
        let result = Cli::try_parse_from(["agent-console-dashboard", "claude-hook", "invalid"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_dump_subcommand_parses() {
        let cli =
            Cli::try_parse_from(["agent-console-dashboard", "dump"]).expect("dump should parse");
        match cli.command {
            Commands::Dump { socket, format } => {
                assert_eq!(socket, PathBuf::from("/tmp/agent-console-dashboard.sock"));
                assert_eq!(format, "json");
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn test_dump_with_format_json() {
        let cli = Cli::try_parse_from(["agent-console-dashboard", "dump", "--format", "json"])
            .expect("dump --format json should parse");
        match cli.command {
            Commands::Dump { format, .. } => {
                assert_eq!(format, "json");
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn test_dump_with_format_text_parses() {
        // CLI accepts any string for format; validation happens at runtime
        let cli = Cli::try_parse_from(["agent-console-dashboard", "dump", "--format", "text"])
            .expect("dump --format text should parse");
        match cli.command {
            Commands::Dump { format, .. } => {
                assert_eq!(format, "text");
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn test_dump_with_custom_socket() {
        let cli = Cli::try_parse_from([
            "agent-console-dashboard",
            "dump",
            "--socket",
            "/custom/dump.sock",
        ])
        .expect("dump --socket should parse");
        match cli.command {
            Commands::Dump { socket, .. } => {
                assert_eq!(socket, PathBuf::from("/custom/dump.sock"));
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn test_resurrect_subcommand_parses() {
        let cli = Cli::try_parse_from(["agent-console-dashboard", "resurrect", "session-abc"])
            .expect("resurrect should parse");
        match cli.command {
            Commands::Resurrect {
                session_id, quiet, ..
            } => {
                assert_eq!(session_id, "session-abc");
                assert!(!quiet);
            }
            _ => panic!("expected Resurrect command"),
        }
    }

    #[test]
    fn test_resurrect_quiet_flag() {
        let cli = Cli::try_parse_from([
            "agent-console-dashboard",
            "resurrect",
            "session-abc",
            "--quiet",
        ])
        .expect("resurrect --quiet should parse");
        match cli.command {
            Commands::Resurrect { quiet, .. } => {
                assert!(quiet);
            }
            _ => panic!("expected Resurrect command"),
        }
    }

    #[test]
    fn test_resurrect_requires_session_id() {
        let result = Cli::try_parse_from(["agent-console-dashboard", "resurrect"]);
        assert!(result.is_err());
    }

    // -- Config subcommand --------------------------------------------------

    #[test]
    fn test_config_init_parses() {
        let cli = Cli::try_parse_from(["agent-console-dashboard", "config", "init"])
            .expect("config init should parse");
        match cli.command {
            Commands::Config { action } => match action {
                ConfigAction::Init { force } => assert!(!force),
                _ => panic!("expected Init action"),
            },
            _ => panic!("expected Config command"),
        }
    }

    #[test]
    fn test_config_init_force_parses() {
        let cli = Cli::try_parse_from(["agent-console-dashboard", "config", "init", "--force"])
            .expect("config init --force should parse");
        match cli.command {
            Commands::Config { action } => match action {
                ConfigAction::Init { force } => assert!(force),
                _ => panic!("expected Init action"),
            },
            _ => panic!("expected Config command"),
        }
    }

    #[test]
    fn test_config_path_parses() {
        let cli = Cli::try_parse_from(["agent-console-dashboard", "config", "path"])
            .expect("config path should parse");
        match cli.command {
            Commands::Config { action } => match action {
                ConfigAction::Path => {}
                _ => panic!("expected Path action"),
            },
            _ => panic!("expected Config command"),
        }
    }

    #[test]
    fn test_config_validate_parses() {
        let cli = Cli::try_parse_from(["agent-console-dashboard", "config", "validate"])
            .expect("config validate should parse");
        match cli.command {
            Commands::Config { action } => match action {
                ConfigAction::Validate => {}
                _ => panic!("expected Validate action"),
            },
            _ => panic!("expected Config command"),
        }
    }

    #[test]
    fn test_config_without_action_fails() {
        let result = Cli::try_parse_from(["agent-console-dashboard", "config"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_subcommand_in_help() {
        let cmd = Cli::command();
        let config_cmd = cmd.get_subcommands().find(|sc| sc.get_name() == "config");
        assert!(config_cmd.is_some(), "config subcommand should exist");
    }

    #[test]
    fn test_config_show_parses() {
        let cli = Cli::try_parse_from(["agent-console-dashboard", "config", "show"])
            .expect("config show should parse");
        match cli.command {
            Commands::Config { action } => match action {
                ConfigAction::Show => {}
                _ => panic!("expected Show action"),
            },
            _ => panic!("expected Config command"),
        }
    }

    // -- Install/Uninstall subcommands ----------------------------------------

    #[test]
    fn test_install_subcommand_parses() {
        let cli = Cli::try_parse_from(["agent-console-dashboard", "install"])
            .expect("install should parse");
        assert!(matches!(cli.command, Commands::Install));
    }

    #[test]
    fn test_uninstall_subcommand_parses() {
        let cli = Cli::try_parse_from(["agent-console-dashboard", "uninstall"])
            .expect("uninstall should parse");
        assert!(matches!(cli.command, Commands::Uninstall));
    }

    #[test]
    fn test_acd_hook_definitions_has_seven_entries() {
        let defs = acd_hook_definitions();
        // 7 hooks: PostToolUse removed for experiment (acd-ws6)
        assert_eq!(
            defs.len(),
            7,
            "should define 7 hooks (PostToolUse removed for experiment)"
        );
    }

    #[test]
    fn test_acd_hook_definitions_all_use_acd_command() {
        let defs = acd_hook_definitions();
        for (_, command, _) in &defs {
            assert!(
                command.starts_with("acd claude-hook "),
                "hook command should start with 'acd claude-hook': {}",
                command
            );
        }
    }

    #[test]
    fn test_acd_hook_definitions_notification_hooks_have_matchers() {
        let defs = acd_hook_definitions();
        let notification_hooks: Vec<_> = defs
            .iter()
            .filter(|(event, _, _)| *event == claude_hooks::HookEvent::Notification)
            .collect();
        assert_eq!(
            notification_hooks.len(),
            2,
            "should have 2 Notification hooks"
        );
        for (_, _, matcher) in &notification_hooks {
            assert!(matcher.is_some(), "Notification hooks must have a matcher");
        }
    }

    #[test]
    fn test_acd_hook_definitions_includes_pre_tool_use() {
        let defs = acd_hook_definitions();
        let has_pre_tool_use = defs
            .iter()
            .any(|(event, _, _)| *event == claude_hooks::HookEvent::PreToolUse);
        assert!(has_pre_tool_use, "should have PreToolUse hook");
        // PostToolUse removed for experiment (acd-ws6)
        let has_post_tool_use = defs
            .iter()
            .any(|(event, _, _)| *event == claude_hooks::HookEvent::PostToolUse);
        assert!(
            !has_post_tool_use,
            "PostToolUse should be absent (acd-ws6 experiment)"
        );
    }

    #[test]
    fn test_claude_hook_question_parses() {
        let cli = Cli::try_parse_from(["agent-console-dashboard", "claude-hook", "question"])
            .expect("claude-hook question should parse");
        match cli.command {
            Commands::ClaudeHook { status, .. } => {
                assert_eq!(status, agent_console_dashboard::Status::Question);
            }
            _ => panic!("expected ClaudeHook command"),
        }
    }

    #[test]
    fn test_claude_hook_closed_parses() {
        let cli = Cli::try_parse_from(["agent-console-dashboard", "claude-hook", "closed"])
            .expect("claude-hook closed should parse");
        match cli.command {
            Commands::ClaudeHook { status, .. } => {
                assert_eq!(status, agent_console_dashboard::Status::Closed);
            }
            _ => panic!("expected ClaudeHook command"),
        }
    }

    #[test]
    fn test_validate_hook_input_valid() {
        let input = HookInput {
            session_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            cwd: "/home/user/project".to_string(),
        };
        let warnings = validate_hook_input(&input);
        assert!(warnings.is_empty(), "valid input should have no warnings");
    }

    #[test]
    fn test_validate_hook_input_invalid_session_id_length() {
        let input = HookInput {
            session_id: "short".to_string(),
            cwd: "/home/user/project".to_string(),
        };
        let warnings = validate_hook_input(&input);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("session_id length is 5"));
        assert!(warnings[0].contains("(expected 36)"));
    }

    #[test]
    fn test_validate_hook_input_invalid_session_id_chars() {
        let input = HookInput {
            session_id: "550e8400-e29b-41d4-a716-44665544000G".to_string(),
            cwd: "/home/user/project".to_string(),
        };
        let warnings = validate_hook_input(&input);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("session_id contains invalid characters"));
    }

    #[test]
    fn test_validate_hook_input_empty_cwd() {
        let input = HookInput {
            session_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            cwd: "".to_string(),
        };
        let warnings = validate_hook_input(&input);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("cwd is empty"));
    }

    #[test]
    fn test_validate_hook_input_relative_cwd() {
        let input = HookInput {
            session_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            cwd: "relative/path".to_string(),
        };
        let warnings = validate_hook_input(&input);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("cwd is not an absolute path"));
        assert!(warnings[0].contains("relative/path"));
    }

    #[test]
    fn test_validate_hook_input_multiple_invalid_fields() {
        let input = HookInput {
            session_id: "short".to_string(),
            cwd: "relative".to_string(),
        };
        let warnings = validate_hook_input(&input);
        assert_eq!(warnings.len(), 2);
        assert!(warnings.iter().any(|w| w.contains("session_id")));
        assert!(warnings.iter().any(|w| w.contains("cwd")));
    }

    #[test]
    fn test_validate_hook_input_uppercase_hex_valid() {
        let input = HookInput {
            session_id: "550E8400-E29B-41D4-A716-446655440000".to_string(),
            cwd: "/home/user/project".to_string(),
        };
        let warnings = validate_hook_input(&input);
        assert!(warnings.is_empty(), "uppercase hex should be valid");
    }

    #[test]
    fn test_validate_hook_input_all_dashes_weird_but_passes() {
        let input = HookInput {
            session_id: "------------------------------------".to_string(),
            cwd: "/home/user/project".to_string(),
        };
        let warnings = validate_hook_input(&input);
        assert!(warnings.is_empty(), "36 dashes passes charset validation");
    }

    #[test]
    fn test_validate_hook_input_cwd_with_spaces_valid() {
        let input = HookInput {
            session_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            cwd: "/home/user/my project".to_string(),
        };
        let warnings = validate_hook_input(&input);
        assert!(warnings.is_empty(), "absolute path with spaces is valid");
    }
}
