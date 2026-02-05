# Epic: Logging and Diagnostics

**Epic ID:** E012 **Status:** In Progress **Priority:** High **Estimated
Effort:** S

## Summary

Implement structured logging and diagnostic tooling for the daemon process to
support debugging, monitoring, and operational visibility. This provides the
observability layer needed for a long-running daemon in production use.

## Goals

- Add structured logging to the daemon with configurable log levels
- Provide diagnostic commands for inspecting daemon state
- Enable log output to file for post-mortem analysis
- Support health check mechanism for daemon status

## User Value

Users running the daemon as a background service need visibility into its
behavior when issues arise. Structured logs help diagnose connection failures,
hook processing errors, and unexpected state transitions. Diagnostic commands
enable quick health checks without parsing log files, reducing time spent
troubleshooting.

## Stories

| Story ID                                                 | Title                            | Priority | Status |
| -------------------------------------------------------- | -------------------------------- | -------- | ------ |
| [S012.01](../stories/S012.01-structured-logging.md)      | Add structured logging to daemon | P1       | Done   |
| [S012.02](../stories/S012.02-health-check-command.md)    | Implement health check command   | P2       | Draft  |
| [S012.03](../stories/S012.03-diagnostic-dump-command.md) | Add diagnostic dump command      | P3       | Draft  |

## Dependencies

- [E001 - Daemon Core Infrastructure](./E001-daemon-core-infrastructure.md) -
  Daemon must exist to add logging to
- [E007 - Configuration System](./E007-configuration-system.md) - Log level and
  output path configured via config file

## Acceptance Criteria

- [ ] Daemon logs startup, shutdown, and connection events at `info` level
- [ ] Hook processing and state changes logged at `debug` level
- [ ] Errors logged with context (session ID, command, source)
- [ ] Log level configurable via config file and environment variable
      (`ACD_LOG`)
- [ ] Log output to stderr (foreground) or file (background/daemonized)
- [ ] `acd status` command returns daemon health: uptime, session count,
      connection count, memory usage
- [ ] `acd dump` command outputs full daemon state as JSON for debugging
- [ ] Unit tests for log formatting per
      [testing strategy](../decisions/testing-strategy.md)

## Technical Notes

### Logging Framework

Use the `tracing` crate (standard Rust ecosystem choice):

```rust
use tracing::{info, debug, warn, error};

info!(session_id = %id, status = %status, "session status updated");
warn!(error = %err, "failed to process hook command");
```

### Configuration

```toml
[daemon]
log_level = "info"       # trace, debug, info, warn, error
log_file = ""            # empty = stderr, path = file output
```

Environment variable override: `ACD_LOG=debug acd daemon`

### Diagnostic Commands

```bash
# Health check (exit code 0 = healthy)
acd status

# Full state dump
acd dump
acd dump --format json    # machine-readable
```

### Health Check Output

```text
Agent Console Daemon
  Status:      running
  Uptime:      2h 34m
  Sessions:    3 active, 1 closed
  Connections: 2 dashboards
  Memory:      2.1 MB
  Socket:      /tmp/acd.sock
```

### Log Format

Two output formats depending on target:

| Target              | Format          | Example                                                                                             |
| ------------------- | --------------- | --------------------------------------------------------------------------------------------------- |
| stderr (foreground) | Human-readable  | `2026-01-31T12:00:00 INFO session status updated session_id=abc123`                                 |
| file (background)   | JSON structured | `{"ts":"2026-01-31T12:00:00Z","level":"info","msg":"session status updated","session_id":"abc123"}` |

Use `tracing-subscriber` with `fmt::Layer` for stderr and `json::Layer` for file
output. Both layers can be active simultaneously.

### Log File Permissions

Log files created with mode `0600` (owner read/write only) since they may
contain session IDs and working directory paths.

### Memory Measurement

The `acd status` health check reports memory usage. Use the `sysinfo` crate to
query resident set size of the daemon process. This is a best-effort metric — if
`sysinfo` fails, report "N/A".

### Log Rotation

Out of scope for v0. Users can use standard tools (`logrotate`, `newsyslog`) for
file-based logs. Log file path change requires daemon restart per
[E007 hot-reload scope](./E007-configuration-system.md).

## Out of Scope

- Metrics export (Prometheus, OpenTelemetry) — deferred to v2+
- Remote logging — deferred to v2+
- Log rotation — use OS tools
