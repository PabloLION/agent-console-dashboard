//! Agent Console Dashboard - CLI entry point
//!
//! This binary provides the command-line interface for the Agent Console daemon.
//! It supports running in foreground or daemonized mode with configurable socket paths.

#[cfg(test)]
mod cli_tests;
mod commands;

use agent_console_dashboard::{
    daemon::run_daemon,
    tui::app::{App, LayoutMode},
    DaemonConfig, Status,
};
use clap::{Parser, Subcommand, ValueEnum};
use commands::{
    is_daemon_running, run_claude_hook_async, run_config_edit_command, run_daemon_stop_command,
    run_delete_command, run_dump_command, run_install_command, run_status_command,
    run_uninstall_command, run_update_command, HookInput,
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

/// CLI-compatible layout mode values for the --layout flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lowercase")]
enum LayoutModeArg {
    /// Auto-detect layout based on terminal height (default).
    Auto,
    /// Full dashboard layout with session list and detail panel.
    Large,
    /// Compact two-line layout for narrow terminals.
    TwoLine,
}

/// Available subcommands for the agent-console CLI
#[derive(Subcommand)]
enum Commands {
    /// Launch the terminal user interface
    Tui {
        /// Socket path for IPC communication
        #[arg(long, default_value = "/tmp/agent-console-dashboard.sock")]
        socket: PathBuf,
        /// Layout mode (large or twoline)
        #[arg(long, value_enum, ignore_case = true)]
        layout: Option<LayoutModeArg>,
    },

    /// Manage configuration file
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Session management
    Session {
        #[command(subcommand)]
        command: SessionCommands,
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

/// Session management subcommands
#[derive(Subcommand)]
enum SessionCommands {
    /// Update session fields (status, priority, working directory)
    Update {
        /// Session ID
        id: String,
        /// Status (working, attention, question, closed)
        #[arg(long)]
        status: Option<String>,
        /// Session priority for sorting (higher = ranked higher)
        #[arg(long)]
        priority: Option<u64>,
        /// Working directory
        #[arg(long)]
        working_dir: Option<PathBuf>,
        /// Socket path for IPC communication
        #[arg(long, default_value = "/tmp/agent-console-dashboard.sock")]
        socket: PathBuf,
    },
    /// Delete a session by ID
    Delete {
        /// Session ID
        session_id: String,
        /// Socket path for IPC communication
        #[arg(long, default_value = "/tmp/agent-console-dashboard.sock")]
        socket: PathBuf,
    },
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
    /// Restart the daemon (stop with force, then start)
    Restart {
        /// Socket path for IPC communication
        #[arg(long, default_value = "/tmp/agent-console-dashboard.sock")]
        socket: PathBuf,
        /// Run daemon in background (detach from terminal)
        #[arg(short, long)]
        detach: bool,
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
        Commands::Tui { socket, layout } => {
            let rt =
                tokio::runtime::Runtime::new().expect("failed to create tokio runtime for TUI");
            if let Err(e) = rt.block_on(async {
                let layout_mode_override = layout.and_then(|l| match l {
                    LayoutModeArg::Auto => None,
                    LayoutModeArg::Large => Some(LayoutMode::Large),
                    LayoutModeArg::TwoLine => Some(LayoutMode::TwoLine),
                });
                let mut app = App::new(socket, layout_mode_override);
                // Wire hooks from config if available
                if let Ok(config) =
                    agent_console_dashboard::config::loader::ConfigLoader::load_default()
                {
                    app.activate_hooks = config.tui.activate_hooks;
                    app.reopen_hooks = config.tui.reopen_hooks;
                }
                app.run().await
            }) {
                eprintln!("TUI error: {}", e);
                return ExitCode::FAILURE;
            }
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
        Commands::Session { command } => match command {
            SessionCommands::Update {
                id,
                status,
                priority,
                working_dir,
                socket,
            } => {
                return run_update_command(
                    &socket,
                    &id,
                    status.as_deref(),
                    working_dir.as_deref(),
                    priority,
                );
            }
            SessionCommands::Delete { session_id, socket } => {
                return run_delete_command(&socket, &session_id);
            }
        },
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
            DaemonCommands::Restart { socket, detach } => {
                // Stop daemon with force=true (skip confirmation)
                if is_daemon_running(&socket) {
                    let stop_exit = run_daemon_stop_command(&socket, true);
                    if stop_exit != ExitCode::SUCCESS {
                        eprintln!("Error: failed to stop daemon during restart");
                        return ExitCode::FAILURE;
                    }
                }

                // Start daemon with same socket path and detach flag
                let config = DaemonConfig::new(socket, detach);
                if let Err(e) = run_daemon(config) {
                    eprintln!("Error: failed to start daemon during restart: {}", e);
                    return ExitCode::FAILURE;
                }
            }
            DaemonCommands::Status { socket } => {
                return run_status_command(&socket);
            }
            DaemonCommands::Dump { socket, format } => {
                if format != "json" {
                    eprintln!(
                        "Error: format '{}' not yet implemented, only 'json' is supported",
                        format
                    );
                    return ExitCode::FAILURE;
                }
                return run_dump_command(&socket);
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
