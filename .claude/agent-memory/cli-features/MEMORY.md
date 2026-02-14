# CLI Features Agent Memory

## Implementation Patterns

### Adding New Daemon Subcommands

When adding a new daemon subcommand (like `acd daemon restart`):

1. Add variant to `DaemonCommands` enum in `src/main.rs` with clap attributes
2. Add match arm in `Commands::Daemon` handler in `src/main.rs`
3. Reuse existing functions from `src/commands/daemon.rs` when possible
4. Add CLI parsing tests in `src/cli_tests/cli.rs`

Example: `acd daemon restart` reuses `run_daemon_stop_command` with `force=true`
and `run_daemon` for start logic.

### Restart Command Implementation (acd-11x)

The restart command follows the pattern:

- Stop daemon with force=true (skips confirmation)
- If daemon not running, just start it (don't error)
- Start daemon with same socket path and detach flag
- Useful for changing env vars like `ACD_LOG` that only take effect at startup

### Test Patterns

CLI tests use `Cli::try_parse_from()` to verify argument parsing without
actually running commands. Tests verify:

- Command variants match expected enum
- Default values are correct
- Flags are properly parsed
- Help metadata exists

## File Organization

- `src/main.rs` - CLI entry point, clap definitions
- `src/commands/mod.rs` - Command module exports
- `src/commands/daemon.rs` - Daemon lifecycle commands (start, stop, helpers)
- `src/cli_tests/cli.rs` - CLI parsing tests (43+ tests)

## Key Functions

- `is_daemon_running(&socket)` - Check if daemon is reachable via socket
- `run_daemon_stop_command(&socket, force)` - Stop daemon with optional
  confirmation
- `run_daemon(config)` - Start daemon with DaemonConfig

## Convention Notes

- Default socket: `/tmp/agent-console-dashboard.sock`
- All daemon commands accept `--socket` flag
- `--detach` flag runs daemon in background
- Tests must not hardcode version numbers - use `env!("CARGO_PKG_VERSION")`
