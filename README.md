# Agent Console Dashboard

Real-time TUI dashboard for monitoring Claude Code sessions.

## Quick Start

```sh
# Install the binary
cargo install --path crates/agent-console-dashboard

# Register hooks with Claude Code
acd install

# Start the dashboard
acd tui
```

## Setup

### Install Hooks

ACD integrates with Claude Code via hooks. Run `acd install` to register all
hooks in `~/.claude/settings.json`:

```sh
acd install
```

This registers hooks for session lifecycle events (start, stop, prompt submit)
and tool use events. The daemon starts automatically when a hook fires.

To remove hooks:

```sh
acd uninstall
```

### Configuration

Create a default configuration file:

```sh
acd config init
```

View or validate the current configuration:

```sh
acd config show
acd config validate
```

Configuration path: `~/.config/agent-console-dashboard/config.toml`

## Usage

### TUI Dashboard

```sh
acd tui
```

Navigate with `j`/`k` or arrow keys. Press `Enter` to view session details.
Press `q` to quit.

### Daemon Management

```sh
acd daemon start           # Start in foreground
acd daemon start --detach  # Start in background
acd daemon stop            # Stop the daemon
acd daemon status          # Check daemon health
acd daemon dump            # Export all sessions as JSON
```

### Session Commands

```sh
acd session update <id> --status=working   # Update session status
acd session update <id> --priority=5       # Set session priority
```

## Development

See [Development Scripts](scripts/README.md) for available commands.

```sh
./scripts/test.sh   # Run tests
./scripts/lint.sh   # Check formatting + clippy
./scripts/fmt.sh    # Auto-fix formatting
```

## License

MIT
