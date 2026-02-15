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

### Removing CLI Commands (acd-jau)

When removing a CLI command:

1. Remove enum variant from `Commands` in `src/main.rs`
2. Remove match arm handler in `src/main.rs`
3. Remove function import from `use` statement in `src/main.rs`
4. Remove function implementation from `src/commands/<module>.rs`
5. Remove related tests from `src/cli_tests/cli.rs`
6. Update module doc comments in `src/commands/mod.rs` and affected module files

Pattern: CLI command removal is straightforward - no complex dependencies.
Daemon/TUI functionality may remain (separate concerns).

### Config Schema Changes (acd-puk)

When renaming or adding config fields:

1. Update struct fields in `src/config/schema.rs` with doc comments
2. Update Default impl to initialize new fields
3. Update tests - rename existing tests and add new ones for new fields
4. Update `DEFAULT_CONFIG_TEMPLATE` in `src/config/default.rs`
5. Check for field references outside config module (e.g., main.rs TUI setup)

Pattern: Config schema changes may require minimal updates to config consumers
(like main.rs) even when TUI logic changes are separate. Add TODO comments
referencing the follow-up issue for full integration.

### CLI Tree Restructuring (acd-2jp)

When restructuring the CLI command hierarchy:

1. Create new subcommand enum (e.g., `SessionCommands`) with proper clap
   attributes
2. Move command variants from `Commands` to new enum, adjusting field types as
   needed (positional → optional flags)
3. Update `Commands` enum to reference new subcommand enum
4. Update match arms in `main()` to handle new command structure
5. Rename command handler functions in `src/commands/*.rs` if semantics changed
6. Update handler function signatures to match new optional parameters
7. Update tests in `src/cli_tests/cli.rs` - add new enum to imports, rewrite
   test assertions for new command paths
8. Update integration tests in `tests/*.rs` - change command invocations to new
   syntax

Pattern: Integration tests use `assert_cmd::Command` with `.args([...])` to
invoke the CLI. Update all call sites when command paths change. Tests in
`tests/` directory are separate from unit tests in `src/`.

Key insight: When making a field optional (e.g., status becomes `Option<&str>`),
update handler to check if any fields provided and warn if none. This prevents
silent no-ops.

### Enhancing Uninstall Command (acd-lj1)

When enhancing `run_uninstall_command()` to clean up the full system:

1. Hook removal (existing functionality, preserve)
2. Stop daemon: use `is_daemon_running()` and
   `run_daemon_stop_command(&socket, true)` with force=true
3. Remove socket file: use `agent_console_dashboard::config::xdg::socket_path()`
   and `std::fs::remove_file()`
4. Print config path: use `agent_console_dashboard::config::xdg::config_path()`
   but do NOT delete

Pattern: Use `agent_console_dashboard::` prefix for library imports from binary
crate, not `crate::`. The binary crate (`src/main.rs`) imports from the library
crate. Use graceful failures (warnings) for daemon stop and socket removal to
avoid blocking uninstall on missing resources.
