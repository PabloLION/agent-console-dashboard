//! Agent Console Dashboard - CLI entry point
//!
//! This binary provides the command-line interface for the Agent Console daemon.
//! It supports running in foreground or daemonized mode with configurable socket paths.

use agent_console_dashboard::{
    client::connect_with_lazy_start, daemon::run_daemon, format_uptime, tui::app::App,
    DaemonConfig, DaemonDump, HealthStatus, Status,
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

    /// Start the daemon process
    Daemon {
        /// Run as a background daemon (detached from terminal)
        #[arg(long)]
        daemonize: bool,

        /// Socket path for IPC communication
        #[arg(long, default_value = "/tmp/agent-console-dashboard.sock")]
        socket: PathBuf,
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
                    Ok(path) => {
                        println!("Created configuration at {}", path.display());
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
        Commands::Daemon { daemonize, socket } => {
            // Create DaemonConfig from CLI args
            let config = DaemonConfig::new(socket, daemonize);

            // Run the daemon - this will:
            // 1. Call daemonize_process() if --daemonize flag set
            // 2. Start Tokio runtime AFTER daemonization
            // 3. Wait for shutdown signal (SIGINT/SIGTERM)
            if let Err(e) = run_daemon(config) {
                eprintln!("Error: {}", e);
                return ExitCode::FAILURE;
            }
        }
        Commands::ClaudeHook { status, socket } => {
            // Parse stdin synchronously before creating async runtime
            let input: HookInput = match serde_json::from_reader(std::io::stdin()) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("acd claude-hook: failed to parse JSON from stdin: {}", e);
                    return ExitCode::from(2);
                }
            };

            let rt = tokio::runtime::Runtime::new()
                .expect("failed to create tokio runtime for hook");
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

/// Connects to daemon, sends SET command to create/update a session.
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
    let cmd = format!("SET {} {} {}\n", session_id, status, wd);

    if writer.write_all(cmd.as_bytes()).is_err() || writer.flush().is_err() {
        eprintln!("Error: failed to send SET command");
        return ExitCode::FAILURE;
    }

    let mut response = String::new();
    if reader.read_line(&mut response).is_err() {
        eprintln!("Error: failed to read daemon response");
        return ExitCode::FAILURE;
    }

    let trimmed = response.trim();
    if trimmed.starts_with("OK") {
        ExitCode::SUCCESS
    } else {
        eprintln!("{}", trimmed);
        ExitCode::FAILURE
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

/// Connects to daemon via lazy-start (spawning if needed), sends SET command.
///
/// Exit codes per Claude Code hook spec:
/// - 0: success (outputs `{"continue": true}` on stdout)
///
/// This function never returns a non-zero exit code after stdin parsing
/// succeeds — hook failures are reported via systemMessage to avoid blocking
/// Claude Code.
async fn run_claude_hook_async(
    socket: &std::path::Path,
    status: Status,
    input: &HookInput,
) -> ExitCode {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

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

    let cmd = format!("SET {} {} {}\n", input.session_id, status, input.cwd);

    if writer.write_all(cmd.as_bytes()).await.is_err() || writer.flush().await.is_err() {
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

    if line.trim().starts_with("OK") {
        println!(r#"{{"continue": true}}"#);
    } else {
        let json = serde_json::json!({
            "continue": true,
            "systemMessage": format!(
                "acd daemon error: {}, session {} not tracked",
                line.trim(), input.session_id
            ),
        });
        println!("{}", json);
    }
    ExitCode::SUCCESS
}

/// Connects to the daemon socket, sends STATUS, and displays health info.
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

    if writer.write_all(b"STATUS\n").is_err() || writer.flush().is_err() {
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

    let trimmed = response.trim();
    if let Some(json_str) = trimmed.strip_prefix("OK ") {
        match serde_json::from_str::<HealthStatus>(json_str) {
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
                eprintln!("Failed to parse daemon response: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    eprintln!("Unexpected daemon response: {}", trimmed);
    ExitCode::FAILURE
}

/// Connects to the daemon socket, sends DUMP, and prints raw JSON.
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

    if writer.write_all(b"DUMP\n").is_err() || writer.flush().is_err() {
        eprintln!("Error: failed to send DUMP command");
        return ExitCode::FAILURE;
    }

    let mut response = String::new();
    if reader.read_line(&mut response).is_err() {
        eprintln!("Error: failed to read daemon response");
        return ExitCode::FAILURE;
    }

    let trimmed = response.trim();
    if let Some(json_str) = trimmed.strip_prefix("OK ") {
        // Validate it parses, then print raw JSON
        match serde_json::from_str::<DaemonDump>(json_str) {
            Ok(_) => {
                println!("{}", json_str);
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("Failed to parse daemon response: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    eprintln!("Unexpected daemon response: {}", trimmed);
    ExitCode::FAILURE
}

/// Connects to daemon, sends RESURRECT, and displays resurrection metadata.
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

    let cmd = format!("RESURRECT {}\n", session_id);
    if writer.write_all(cmd.as_bytes()).is_err() || writer.flush().is_err() {
        eprintln!("Error: failed to send RESURRECT command");
        return ExitCode::FAILURE;
    }

    let mut response = String::new();
    if reader.read_line(&mut response).is_err() {
        eprintln!("Error: failed to read daemon response");
        return ExitCode::FAILURE;
    }

    let trimmed = response.trim();
    if let Some(json_str) = trimmed.strip_prefix("OK ") {
        match serde_json::from_str::<serde_json::Value>(json_str) {
            Ok(info) => {
                let sid = info["session_id"].as_str().unwrap_or(session_id);
                let wd = info["working_dir"].as_str().unwrap_or("<unknown>");
                let cmd = info["command"].as_str().unwrap_or("claude --resume");
                if quiet {
                    println!("cd {} && {}", wd, cmd);
                } else {
                    println!("To resurrect session {}:", sid);
                    println!("  cd {}", wd);
                    println!("  {}", cmd);
                }
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("Failed to parse daemon response: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    if let Some(err_msg) = trimmed.strip_prefix("ERR ") {
        eprintln!("Error: {}", err_msg);
    } else {
        eprintln!("Unexpected daemon response: {}", trimmed);
    }
    ExitCode::FAILURE
}

/// Returns the complete list of ACD hooks to install.
///
/// Each entry: (event, command, timeout, matcher).
/// This is the single source of truth for which hooks ACD registers.
fn acd_hook_definitions() -> Vec<(claude_hooks::HookEvent, &'static str, Option<String>)> {
    use claude_hooks::HookEvent;
    vec![
        (HookEvent::SessionStart, "acd claude-hook attention", None),
        (
            HookEvent::UserPromptSubmit,
            "acd claude-hook working",
            None,
        ),
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
        println!("Restart Claude Code for hooks to take effect.");
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
        println!("Restart Claude Code for changes to take effect.");
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
    fn test_daemon_subcommand_exists() {
        // Verify the daemon subcommand can be parsed
        let result = Cli::try_parse_from(["agent-console-dashboard", "daemon"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_default_socket_path() {
        // Verify default socket path is /tmp/agent-console-dashboard.sock
        let cli = Cli::try_parse_from(["agent-console-dashboard", "daemon"]).unwrap();
        match cli.command {
            Commands::Daemon { socket, .. } => {
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
            "--socket",
            "/custom/path.sock",
        ])
        .unwrap();
        match cli.command {
            Commands::Daemon { socket, .. } => {
                assert_eq!(socket, PathBuf::from("/custom/path.sock"));
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn test_daemonize_flag_default_false() {
        // Verify daemonize flag defaults to false
        let cli = Cli::try_parse_from(["agent-console-dashboard", "daemon"]).unwrap();
        match cli.command {
            Commands::Daemon { daemonize, .. } => {
                assert!(!daemonize);
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn test_daemonize_flag_true() {
        // Verify daemonize flag can be set to true
        let cli =
            Cli::try_parse_from(["agent-console-dashboard", "daemon", "--daemonize"]).unwrap();
        match cli.command {
            Commands::Daemon { daemonize, .. } => {
                assert!(daemonize);
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn test_daemon_help_contains_expected_options() {
        // Verify that the daemon subcommand help contains --daemonize and --socket
        let cmd = Cli::command();
        let daemon_cmd = cmd
            .get_subcommands()
            .find(|sc| sc.get_name() == "daemon")
            .expect("daemon subcommand should exist");

        // Check that --daemonize option exists
        let daemonize_arg = daemon_cmd
            .get_arguments()
            .find(|arg| arg.get_id() == "daemonize");
        assert!(daemonize_arg.is_some(), "--daemonize flag should exist");

        // Check that --socket option exists
        let socket_arg = daemon_cmd
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
            "--daemonize",
            "--socket",
            "/var/run/my-daemon.sock",
        ])
        .unwrap();
        match cli.command {
            Commands::Daemon { daemonize, socket } => {
                assert!(daemonize);
                assert_eq!(socket, PathBuf::from("/var/run/my-daemon.sock"));
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn test_flag_order_independence() {
        // Verify flags can be specified in any order (--socket before --daemonize)
        let cli = Cli::try_parse_from([
            "agent-console-dashboard",
            "daemon",
            "--socket",
            "/custom/path.sock",
            "--daemonize",
        ])
        .unwrap();
        match cli.command {
            Commands::Daemon { daemonize, socket } => {
                assert!(daemonize);
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
        let result = Cli::try_parse_from(["agent-console-dashboard", "daemon", "--socket"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_socket_path_with_spaces() {
        // Verify socket path with spaces works correctly
        let cli = Cli::try_parse_from([
            "agent-console-dashboard",
            "daemon",
            "--socket",
            "/path/with spaces/socket.sock",
        ])
        .unwrap();
        match cli.command {
            Commands::Daemon { socket, .. } => {
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
            "--socket",
            "./local.sock",
        ])
        .unwrap();
        match cli.command {
            Commands::Daemon { socket, .. } => {
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
    fn test_acd_hook_definitions_has_six_entries() {
        let defs = acd_hook_definitions();
        assert_eq!(defs.len(), 6, "should define 6 hooks");
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
        assert_eq!(notification_hooks.len(), 2, "should have 2 Notification hooks");
        for (_, _, matcher) in &notification_hooks {
            assert!(
                matcher.is_some(),
                "Notification hooks must have a matcher"
            );
        }
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
}
