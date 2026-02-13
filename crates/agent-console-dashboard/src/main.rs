//! Agent Console Dashboard - CLI entry point
//!
//! This binary provides the command-line interface for the Agent Console daemon.
//! It supports running in foreground or daemonized mode with configurable socket paths.

mod commands;
#[cfg(test)]
mod tests;

use agent_console_dashboard::{daemon::run_daemon, tui::app::App, DaemonConfig, Status};
use clap::{Parser, Subcommand};
use commands::{
    is_daemon_running, run_claude_hook_async, run_config_edit_command, run_daemon_stop_command,
    run_dump_command, run_install_command, run_resurrect_command, run_set_command,
    run_status_command, run_uninstall_command, HookInput,
};
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
    /// Open configuration file in editor
    Edit,
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
                ConfigAction::Edit => run_config_edit_command(),
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
