# Epic: Daemon Core Infrastructure

**Epic ID:** E001 **Status:** Draft **Priority:** High **Estimated Effort:** L

## Summary

Build the foundational daemon process that serves as the central hub for the
Agent Console Dashboard system. The daemon manages session state in memory,
provides a Unix socket server for IPC, and auto-starts when clients connect.
This is the backbone infrastructure that all other features depend on.

## Goals

- Create a lightweight, long-running daemon process with minimal resource
  footprint
- Implement Unix socket server for reliable local IPC communication
- Build an efficient in-memory session store using HashMap
- Enable auto-start behavior so the daemon launches automatically when needed

## User Value

Users get a reliable, always-available backend service that coordinates all
agent session information. The daemon's push-model architecture ensures
real-time updates without polling, keeping the dashboard instantly responsive to
session state changes. The minimal footprint (<5MB RAM) means it can run
continuously without impacting system performance.

## Stories

| Story ID                                           | Title                                    | Priority | Status |
| -------------------------------------------------- | ---------------------------------------- | -------- | ------ |
| [S1.1](../stories/S1.1-create-daemon-process.md)   | Create daemon process with CLI interface | P1       | Draft  |
| [S1.2](../stories/S1.2-unix-socket-server.md)      | Implement Unix socket server             | P1       | Draft  |
| [S1.3](../stories/S1.3-in-memory-session-store.md) | Implement in-memory session store        | P1       | Draft  |
| [S1.4](../stories/S1.4-daemon-auto-start.md)       | Add daemon auto-start capability         | P2       | Draft  |

## Dependencies

- None (this is the foundational epic)

## Acceptance Criteria

- [ ] Daemon process starts and runs in both foreground and background modes
- [ ] Unix socket server accepts connections at `/tmp/agent-console.sock`
- [ ] Session store correctly manages session state in memory
- [ ] Daemon auto-starts when first client attempts to connect
- [ ] RAM usage stays under 5MB target
- [ ] Startup time is under 100ms

## Technical Notes

### Architecture Decision

The daemon approach was chosen over shared memory and SQLite alternatives:

| Approach      | Rejected Reason                                                    |
| ------------- | ------------------------------------------------------------------ |
| Shared Memory | Requires `unsafe` Rust, platform-specific, complex synchronization |
| SQLite        | Adds 1-2MB to binary, requires polling, persistence not needed     |

### Project Structure

```text
src/
├── main.rs           # CLI entry, argument parsing
├── daemon/
│   ├── mod.rs
│   ├── server.rs     # Socket server
│   ├── store.rs      # State management
│   └── protocol.rs   # IPC message parsing
```

### Key Dependencies

| Crate | Purpose                         |
| ----- | ------------------------------- |
| tokio | Async runtime for socket server |
| clap  | CLI argument parsing            |

### CLI Commands

```bash
# Start daemon (foreground, for development)
agent-console daemon

# Start daemon (background)
agent-console daemon --daemonize

# With custom socket path
agent-console daemon --socket /tmp/agent-console.sock
```

### Success Metrics

| Metric         | Target |
| -------------- | ------ |
| RAM usage      | <5MB   |
| Update latency | <1ms   |
| Binary size    | <10MB  |
| Startup time   | <100ms |
