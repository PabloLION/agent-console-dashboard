//! Agent Console Dashboard - CLI entry point
//!
//! This binary provides the command-line interface for the Agent Console daemon.
//! It supports running in foreground or daemonized mode with configurable socket paths.

use agent_console::{
    daemon::run_daemon, format_uptime, service, tui::app::App, DaemonConfig, DaemonDump,
    HealthStatus,
};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

/// Agent Console Dashboard daemon
#[derive(Parser)]
#[command(name = "agent-console")]
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
        #[arg(long, default_value = "/tmp/agent-console.sock")]
        socket: PathBuf,
    },

    /// Check daemon health status
    Status {
        /// Socket path for IPC communication
        #[arg(long, default_value = "/tmp/agent-console.sock")]
        socket: PathBuf,
    },

    /// Dump full daemon state as JSON
    Dump {
        /// Socket path for IPC communication
        #[arg(long, default_value = "/tmp/agent-console.sock")]
        socket: PathBuf,
        /// Output format (only json supported in v0)
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Manage daemon system service (install/uninstall/status)
    Service {
        #[command(subcommand)]
        action: ServiceAction,
    },

    /// Manage configuration file
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Start the daemon process
    Daemon {
        /// Run as a background daemon (detached from terminal)
        #[arg(long)]
        daemonize: bool,

        /// Socket path for IPC communication
        #[arg(long, default_value = "/tmp/agent-console.sock")]
        socket: PathBuf,
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
}

/// Actions for the `service` subcommand.
#[derive(Subcommand)]
enum ServiceAction {
    /// Install daemon as a system service (launchd on macOS, systemd on Linux)
    Install,
    /// Uninstall daemon system service
    Uninstall,
    /// Check daemon service status
    Status,
}

fn main() -> ExitCode {
    // Parse CLI arguments BEFORE any fork/runtime operations
    // This ensures errors are shown to the user in the terminal
    let cli = Cli::parse();

    match cli.command {
        Commands::Tui { socket } => {
            let rt = tokio::runtime::Runtime::new()
                .expect("failed to create tokio runtime for TUI");
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
        Commands::Config { action } => {
            use agent_console::config::{default, loader::ConfigLoader, xdg};
            let result = match action {
                ConfigAction::Init { force } => {
                    match default::create_default_config(force) {
                        Ok(path) => {
                            println!("Created configuration at {}", path.display());
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                }
                ConfigAction::Path => {
                    println!("{}", xdg::config_path().display());
                    Ok(())
                }
                ConfigAction::Validate => {
                    match ConfigLoader::load_default() {
                        Ok(config) => {
                            println!("Configuration is valid");
                            println!("{config:#?}");
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                }
            };
            if let Err(e) = result {
                eprintln!("Config error: {e}");
                return ExitCode::FAILURE;
            }
        }
        Commands::Service { action } => {
            let result = match action {
                ServiceAction::Install => service::install_service(),
                ServiceAction::Uninstall => service::uninstall_service(),
                ServiceAction::Status => service::service_status(),
            };
            if let Err(e) = result {
                eprintln!("Service error: {e}");
                return ExitCode::FAILURE;
            }
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
        let result = Cli::try_parse_from(["agent-console", "daemon"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_default_socket_path() {
        // Verify default socket path is /tmp/agent-console.sock
        let cli = Cli::try_parse_from(["agent-console", "daemon"]).unwrap();
        match cli.command {
            Commands::Daemon { socket, .. } => {
                assert_eq!(socket, PathBuf::from("/tmp/agent-console.sock"));
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn test_custom_socket_path() {
        // Verify custom socket path can be specified
        let cli = Cli::try_parse_from(["agent-console", "daemon", "--socket", "/custom/path.sock"])
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
        let cli = Cli::try_parse_from(["agent-console", "daemon"]).unwrap();
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
        let cli = Cli::try_parse_from(["agent-console", "daemon", "--daemonize"]).unwrap();
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
            "agent-console",
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
            "agent-console",
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
        let result = Cli::try_parse_from(["agent-console", "unknown"]);
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
        let result = Cli::try_parse_from(["agent-console", "daemon", "--socket"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_socket_path_with_spaces() {
        // Verify socket path with spaces works correctly
        let cli = Cli::try_parse_from([
            "agent-console",
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
        let cli =
            Cli::try_parse_from(["agent-console", "daemon", "--socket", "./local.sock"]).unwrap();
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
        let result = Cli::try_parse_from(["agent-console", "daemon", "--unknown-flag"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_dump_subcommand_parses() {
        let cli = Cli::try_parse_from(["agent-console", "dump"]).expect("dump should parse");
        match cli.command {
            Commands::Dump { socket, format } => {
                assert_eq!(socket, PathBuf::from("/tmp/agent-console.sock"));
                assert_eq!(format, "json");
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn test_dump_with_format_json() {
        let cli = Cli::try_parse_from(["agent-console", "dump", "--format", "json"])
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
        let cli = Cli::try_parse_from(["agent-console", "dump", "--format", "text"])
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
        let cli =
            Cli::try_parse_from(["agent-console", "dump", "--socket", "/custom/dump.sock"])
                .expect("dump --socket should parse");
        match cli.command {
            Commands::Dump { socket, .. } => {
                assert_eq!(socket, PathBuf::from("/custom/dump.sock"));
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn test_service_install_parses() {
        let cli = Cli::try_parse_from(["agent-console", "service", "install"])
            .expect("service install should parse");
        match cli.command {
            Commands::Service { action } => match action {
                ServiceAction::Install => {}
                _ => panic!("expected Install action"),
            },
            _ => panic!("expected Service command"),
        }
    }

    #[test]
    fn test_service_uninstall_parses() {
        let cli = Cli::try_parse_from(["agent-console", "service", "uninstall"])
            .expect("service uninstall should parse");
        match cli.command {
            Commands::Service { action } => match action {
                ServiceAction::Uninstall => {}
                _ => panic!("expected Uninstall action"),
            },
            _ => panic!("expected Service command"),
        }
    }

    #[test]
    fn test_service_status_parses() {
        let cli = Cli::try_parse_from(["agent-console", "service", "status"])
            .expect("service status should parse");
        match cli.command {
            Commands::Service { action } => match action {
                ServiceAction::Status => {}
                _ => panic!("expected Status action"),
            },
            _ => panic!("expected Service command"),
        }
    }

    #[test]
    fn test_service_without_action_fails() {
        let result = Cli::try_parse_from(["agent-console", "service"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_service_unknown_action_fails() {
        let result = Cli::try_parse_from(["agent-console", "service", "restart"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_service_subcommand_in_help() {
        let cmd = Cli::command();
        let service_cmd = cmd
            .get_subcommands()
            .find(|sc| sc.get_name() == "service");
        assert!(service_cmd.is_some(), "service subcommand should exist");
    }

    // -- Config subcommand --------------------------------------------------

    #[test]
    fn test_config_init_parses() {
        let cli = Cli::try_parse_from(["agent-console", "config", "init"])
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
        let cli = Cli::try_parse_from(["agent-console", "config", "init", "--force"])
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
        let cli = Cli::try_parse_from(["agent-console", "config", "path"])
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
        let cli = Cli::try_parse_from(["agent-console", "config", "validate"])
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
        let result = Cli::try_parse_from(["agent-console", "config"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_subcommand_in_help() {
        let cmd = Cli::command();
        let config_cmd = cmd
            .get_subcommands()
            .find(|sc| sc.get_name() == "config");
        assert!(config_cmd.is_some(), "config subcommand should exist");
    }
}
