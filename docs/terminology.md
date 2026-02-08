# Terminology

Project-specific terms used in code and documentation.

## Lazy-start

The daemon starts on demand when a hook or TUI first needs it, rather than
running persistently as a system service (launchd/systemd).

When a client needs the daemon:

1. Attempt socket connection
2. If not running, spawn `acd daemon --daemonize` in the background
3. Retry connection with exponential backoff (10ms initial, 500ms max, 10 retries)
4. Socket binding acts as mutex — concurrent clients won't spawn duplicates

Implementation: `connect_with_lazy_start()` in `crates/agent-console-dashboard/src/client/connection.rs`
