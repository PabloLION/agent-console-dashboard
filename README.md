# Agent Console Dashboard

Terminal dashboard for monitoring Claude Code agent sessions in real-time.

## What It Does

Agent Console Dashboard (ACD) tracks Claude Code sessions and displays their
status in a live terminal UI. It integrates with Claude Code's hook system to
receive session events and shows whether agents are working, waiting for
attention, asking questions, or closed.

## Quick Start

### Install

```sh
cargo install --path crates/agent-console-dashboard
```

### Setup Hooks

Install ACD hooks into Claude Code settings:

```sh
acd install
```

This registers hooks for session lifecycle events (SessionStart,
UserPromptSubmit, Stop, SessionEnd, Notification). Restart Claude Code after
installation.

### Launch Dashboard

```sh
acd tui
```

The daemon starts automatically on first connection. Session status updates
appear in real-time as you interact with Claude Code.

## Commands

### Dashboard

```sh
acd tui                          # Launch interactive dashboard
acd tui --socket /custom.sock    # Use custom socket path
```

**Dashboard controls:**

- `j/k` or `↓/↑` - Navigate sessions
- `Enter` - View session details
- `Esc` - Close detail view
- `1/2` - Switch layout presets
- `q` - Quit

### Daemon Management

```sh
acd daemon                       # Run daemon in foreground (development)
acd daemon --daemonize           # Run daemon in background
acd status                       # Check daemon health
acd dump                         # Export full daemon state (JSON)
```

### Hook Management

```sh
acd install                      # Install hooks to ~/.claude/settings.json
acd uninstall                    # Remove ACD hooks
```

### Session Management

```sh
acd set <session_id> working     # Update session status (called by hooks)
acd resurrect <session_id>       # Get command to resume closed session
```

### Configuration

```sh
acd config init                  # Create default config
acd config init --force          # Recreate config (backs up existing)
acd config path                  # Show config file location
acd config validate              # Validate config syntax
```

## Session States

- **Working** (green) - Agent actively processing
- **Attention** (yellow) - Agent stopped or awaiting user input
- **Question** (blue) - Agent asking interactive question (AskUserQuestion)
- **Closed** (gray) - Session ended
- **Inactive** (dim) - Session idle for more than 1 hour

## Architecture

ACD uses a daemon + client architecture:

```text
┌─────────────────┐
│ Claude Session  │
│    (hooks)      │
└────────┬────────┘
         │ IPC: update <session> <status>
         ▼
┌─────────────────────┐
│  ACD Daemon         │
│  ┌───────────────┐  │
│  │ Session Store │  │  (in-memory)
│  │ State History │  │
│  └───────────────┘  │
│  Unix socket:       │
│  /tmp/agent-console-dashboard.sock
└──────────┬──────────┘
           │ IPC: subscribe / query
           ▼
      ┌─────────┐
      │ TUI     │
      │ Client  │
      └─────────┘
```

**Key components:**

- **Daemon** - Background process managing session state, communicating via Unix
  socket IPC
- **TUI Client** - Terminal dashboard subscribing to daemon updates
- **Hooks** - Claude Code hooks calling `acd claude-hook <status>` to report
  session events
- **Session Store** - In-memory HashMap tracking active and closed sessions

Session state is volatile (not persisted). Daemon restarts clear all session
history.

## Development

### Build

```sh
cargo build                      # Debug build
cargo build --release            # Release build (optimized)
```

### Test

```sh
cargo test                       # Run all tests
cargo test -p agent-console-dashboard  # Test main crate only
cargo test -- --nocapture        # Show test output
```

### Lint

```sh
cargo clippy                     # Lint warnings
cargo clippy -- -D warnings      # Lint as errors
cargo fmt                        # Format code
cargo fmt -- --check             # Check formatting without modifying
```

### Documentation

```sh
cargo doc --open                 # Build and open API docs
```

## Project Structure

```text
agent-console-dashboard/
├── crates/
│   ├── agent-console-dashboard/   # Main binary and daemon
│   │   ├── src/
│   │   │   ├── daemon/            # Daemon server and protocol
│   │   │   ├── client/            # IPC client
│   │   │   ├── tui/               # Terminal UI
│   │   │   ├── config/            # Configuration system
│   │   │   └── integrations/      # Zellij integration
│   │   └── build.rs               # Claude Code plugin generation
│   ├── claude-hooks/              # Hook management library
│   └── claude-usage/              # API usage tracking (future)
└── docs/                          # Architecture and design docs
```

## Configuration

Config file location: `~/.config/agent-console-dashboard/config.toml`

Generate default config:

```sh
acd config init
```

Configuration options documented in the generated file.

## License

See LICENSE file for terms.
