# Story: Create Daemon Process with CLI Interface

**Story ID:** S001
**Epic:** [E001 - Daemon Core Infrastructure](../epic/E001-daemon-core-infrastructure.md)
**Status:** Draft
**Priority:** P1
**Estimated Points:** 5

## Description

As a developer,
I want to create a daemon process with a CLI interface,
So that I have the foundation for the Agent Console backend service.

## Context

The daemon is the central hub of the Agent Console Dashboard system. It needs to run as a long-lived process that can be started in foreground mode (for development/debugging) or daemonized for production use. This story establishes the basic process structure and CLI entry point using clap for argument parsing.

The daemon was chosen over shared memory and SQLite alternatives because:
- Minimal footprint (one socket file, no database)
- Real-time updates via push model
- Safe Rust (no `unsafe` code required)
- Simple data model (HashMap in memory)

## Implementation Details

### Technical Approach

1. Set up project structure with Cargo.toml and dependencies
2. Create `main.rs` with CLI argument parsing using clap
3. Implement daemon subcommand with `--daemonize` and `--socket` flags
4. Create `daemon/mod.rs` as the daemon module entry point
5. Implement basic process lifecycle (start, signal handling, graceful shutdown)
6. Support both foreground and background (daemonized) modes

### Files to Modify

- `Cargo.toml` - Add clap, tokio dependencies
- `src/main.rs` - CLI entry point with argument parsing
- `src/daemon/mod.rs` - Daemon module structure
- `src/lib.rs` - Shared types if needed

### Dependencies

- None (this is the first story in E001)

## Acceptance Criteria

- [ ] Given no daemon running, when `agent-console daemon` is executed, then the process starts in foreground mode
- [ ] Given the `--daemonize` flag, when the daemon starts, then it detaches from the terminal and runs in background
- [ ] Given the `--socket` flag, when the daemon starts, then it uses the specified socket path
- [ ] Given a running daemon, when SIGTERM/SIGINT is received, then the daemon shuts down gracefully
- [ ] Given the binary is built, then the resulting executable is under 10MB
- [ ] Given the daemon starts, then startup time is under 100ms

## Testing Requirements

- [ ] Unit test: CLI argument parsing handles all flags correctly
- [ ] Unit test: Default socket path is `/tmp/agent-console.sock`
- [ ] Integration test: Daemon starts and responds to shutdown signals
- [ ] Integration test: Foreground mode blocks until signal received

## Out of Scope

- Socket server implementation (S002)
- Session store implementation (S003)
- Auto-start capability (S004)
- Any IPC protocol handling

## Notes

### CLI Interface

```bash
# Start daemon (foreground, for development)
agent-console daemon

# Start daemon (background)
agent-console daemon --daemonize

# With custom socket path
agent-console daemon --socket /tmp/agent-console.sock
```

### Project Structure

```text
src/
├── main.rs           # CLI entry, argument parsing
├── daemon/
│   └── mod.rs        # Daemon module entry
└── lib.rs            # Shared types
```

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| tokio | Async runtime for socket server |
| clap | CLI argument parsing |
