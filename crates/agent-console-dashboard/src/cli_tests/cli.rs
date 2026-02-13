//! CLI argument parsing tests.

use crate::{Cli, Commands, ConfigAction, DaemonCommands};
use clap::{CommandFactory, Parser};
use std::path::PathBuf;

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
    let cli =
        Cli::try_parse_from(["agent-console-dashboard", "daemon", "start", "--detach"]).unwrap();
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
    let result = Cli::try_parse_from(["agent-console-dashboard", "daemon", "start", "--socket"]);
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

// -- Dump subcommand ------------------------------------------------

#[test]
fn test_dump_subcommand_parses() {
    let cli = Cli::try_parse_from(["agent-console-dashboard", "dump"]).expect("dump should parse");
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

// -- Resurrect subcommand ------------------------------------------------

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

#[test]
fn test_config_edit_parses() {
    let cli = Cli::try_parse_from(["agent-console-dashboard", "config", "edit"])
        .expect("config edit should parse");
    match cli.command {
        Commands::Config { action } => match action {
            ConfigAction::Edit => {}
            _ => panic!("expected Edit action"),
        },
        _ => panic!("expected Config command"),
    }
}

// -- Install/Uninstall subcommands ----------------------------------------

#[test]
fn test_install_subcommand_parses() {
    let cli =
        Cli::try_parse_from(["agent-console-dashboard", "install"]).expect("install should parse");
    assert!(matches!(cli.command, Commands::Install));
}

#[test]
fn test_uninstall_subcommand_parses() {
    let cli = Cli::try_parse_from(["agent-console-dashboard", "uninstall"])
        .expect("uninstall should parse");
    assert!(matches!(cli.command, Commands::Uninstall));
}
