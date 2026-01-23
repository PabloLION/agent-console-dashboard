//! Agent Console Dashboard - CLI entry point
//!
//! This binary provides the command-line interface for the Agent Console daemon.
//! It supports running in foreground or daemonized mode with configurable socket paths.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

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

fn main() {
    // Parse CLI arguments BEFORE any fork/runtime operations
    // This ensures errors are shown to the user in the terminal
    let cli = Cli::parse();

    match cli.command {
        Commands::Daemon { daemonize, socket } => {
            // For now, just print the configuration
            // Actual daemon logic will be added in subsequent subtasks
            println!("Starting agent-console daemon...");
            println!("  Socket path: {}", socket.display());
            println!("  Daemonize: {}", daemonize);
        }
    }
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
}
