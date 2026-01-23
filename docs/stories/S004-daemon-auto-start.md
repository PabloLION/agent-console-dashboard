# Story: Add Daemon Auto-Start Capability

**Story ID:** S004 **Epic:**
[E001 - Daemon Core Infrastructure](../epic/E001-daemon-core-infrastructure.md)
**Status:** Draft **Priority:** P2 **Estimated Points:** 3

## Description

As a client (hook or dashboard), I want the daemon to start automatically if
it's not running, So that I don't have to manually start the daemon before using
the system.

## Context

For a seamless user experience, clients should not need to manually start the
daemon. When a hook fires or a dashboard launches, if the daemon isn't running,
it should be started automatically. This is the "first client starts the daemon"
pattern.

The auto-start behavior ensures:

- Zero manual setup for users
- Hooks work immediately after installation
- Dashboard can be launched at any time
- System is self-healing (daemon crash = auto-restart on next use)

## Implementation Details

### Technical Approach

1. Create a client library/helper for daemon communication
2. Implement connection attempt with auto-start logic
3. Check if daemon is running (try to connect to socket)
4. If connection fails, spawn daemon process in background
5. Wait for daemon to be ready (retry connection with backoff)
6. Return connected client after daemon is available
7. Add timeout to prevent infinite wait if spawn fails

### Files to Modify

- `src/client/mod.rs` - Client module with auto-start logic
- `src/client/connection.rs` - Connection handling with auto-start

### Dependencies

- S001: Daemon process must support `--daemonize` flag
- S002: Unix socket server must be operational

## Acceptance Criteria

- [ ] Given no daemon running, when a client attempts to connect, then the
      daemon is started automatically
- [ ] Given daemon just started, when client connects, then connection succeeds
      after daemon is ready
- [ ] Given daemon already running, when a client connects, then no new daemon
      is spawned
- [ ] Given auto-start is triggered, when daemon spawns, then it runs in
      background (daemonized)
- [ ] Given daemon fails to start, when timeout is reached, then client gets
      clear error message
- [ ] Given multiple clients try to connect simultaneously, when no daemon
      running, then only one daemon is started

## Testing Requirements

- [ ] Integration test: Client auto-starts daemon when not running
- [ ] Integration test: Second client connects without spawning another daemon
- [ ] Integration test: Connection succeeds after daemon startup delay
- [ ] Integration test: Timeout error when daemon cannot start
- [ ] Integration test: Race condition handling for simultaneous connections

## Out of Scope

- Systemd/launchd service files (user manages service themselves)
- Daemon health monitoring/restart (crash = restart on next use)
- Multiple daemon instances (single instance design)

## Notes

### Auto-Start Flow

```text
┌─────────────────┐
│  Client Start   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐     ┌──────────────────┐
│ Try Connect to  │────►│ Connection OK?   │
│ Socket          │     └────────┬─────────┘
└─────────────────┘              │
                          Yes ───┴─── No
                           │          │
                           ▼          ▼
                    ┌──────────┐ ┌────────────────┐
                    │ Connected│ │ Spawn Daemon   │
                    │ Return   │ │ (daemonized)   │
                    └──────────┘ └───────┬────────┘
                                         │
                                         ▼
                                 ┌───────────────┐
                                 │ Wait + Retry  │
                                 │ (with backoff)│
                                 └───────┬───────┘
                                         │
                              Success ───┴─── Timeout
                                 │              │
                                 ▼              ▼
                          ┌──────────┐  ┌─────────────┐
                          │ Connected│  │ Error:      │
                          │ Return   │  │ Start Failed│
                          └──────────┘  └─────────────┘
```

### Implementation Pattern

```rust
pub async fn connect_with_auto_start(socket_path: &Path) -> Result<Client> {
    // Try to connect first
    match UnixStream::connect(socket_path).await {
        Ok(stream) => return Ok(Client::new(stream)),
        Err(_) => {
            // Daemon not running, try to start it
            spawn_daemon(socket_path)?;

            // Wait for daemon to be ready with exponential backoff
            let mut delay = Duration::from_millis(10);
            for _ in 0..10 {
                tokio::time::sleep(delay).await;
                if let Ok(stream) = UnixStream::connect(socket_path).await {
                    return Ok(Client::new(stream));
                }
                delay = (delay * 2).min(Duration::from_millis(500));
            }

            Err(Error::DaemonStartFailed)
        }
    }
}

fn spawn_daemon(socket_path: &Path) -> Result<()> {
    Command::new(current_exe()?)
        .args(["daemon", "--daemonize", "--socket", socket_path.to_str()?])
        .spawn()?;
    Ok(())
}
```

### Timeout and Retry Configuration

| Parameter           | Value      |
| ------------------- | ---------- |
| Initial retry delay | 10ms       |
| Max retry delay     | 500ms      |
| Max retries         | 10         |
| Total max wait      | ~5 seconds |
