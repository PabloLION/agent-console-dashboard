//! Agent Console Dashboard - CLI entry point
//!
//! This binary provides the command-line interface for the Agent Console daemon.
//! It supports running in foreground or daemonized mode with configurable socket paths.

use agent_console::{daemon::run_daemon, DaemonConfig};
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

fn main() -> ExitCode {
    // Parse CLI arguments BEFORE any fork/runtime operations
    // This ensures errors are shown to the user in the terminal
    let cli = Cli::parse();

    match cli.command {
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
        }
    }

    #[test]
    fn test_custom_socket_path() {
        // Verify custom socket path can be specified
        let cli = Cli::try_parse_from([
            "agent-console",
            "daemon",
            "--socket",
            "/custom/path.sock",
        ])
        .unwrap();
        match cli.command {
            Commands::Daemon { socket, .. } => {
                assert_eq!(socket, PathBuf::from("/custom/path.sock"));
            }
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
        }
    }

    #[test]
    fn test_socket_relative_path() {
        // Verify relative socket path is accepted
        let cli = Cli::try_parse_from([
            "agent-console",
            "daemon",
            "--socket",
            "./local.sock",
        ])
        .unwrap();
        match cli.command {
            Commands::Daemon { socket, .. } => {
                assert_eq!(socket, PathBuf::from("./local.sock"));
            }
        }
    }

    #[test]
    fn test_unknown_flag_fails() {
        // Verify unknown flag fails to parse
        let result = Cli::try_parse_from(["agent-console", "daemon", "--unknown-flag"]);
        assert!(result.is_err());
    }
}
