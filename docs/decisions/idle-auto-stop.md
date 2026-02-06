# Idle Auto-Stop

**Date**: 2026-02-06 **Status**: Accepted **Issue**: acd-2co

## Context

The daemon runs forever until killed (SIGINT/SIGTERM). When no Claude Code
sessions are active, it wastes resources. A mechanism is needed to automatically
stop the daemon after a period of inactivity.

## Decision

Add a periodic idle check in the main event loop that triggers graceful shutdown
after 60 minutes with no active sessions.

### Design

- **`AUTO_STOP_IDLE_SECS = 3600`** — duration before auto-stop (1 hour)
- **`IDLE_CHECK_INTERVAL_SECS = 60`** — check frequency
- Timer starts on daemon boot, so an unused daemon also auto-stops
- Active sessions = any session with `status != Closed`
- Idle check runs via `tokio::select!` alongside the existing signal handler
- State transitions (idle started, timer reset) logged at `info!` level;
  periodic ticks at `debug!`

### Files changed

- `daemon/mod.rs` — constants, `idle_check_loop()`, `tokio::select!` wiring
- `daemon/server.rs` — `store()` getter to expose `SessionStore`

## Alternatives considered

- **Configurable timeout via config.toml** — deferred to acd-jnd (P3). The
  constant is sufficient for now and easier to change later.
- **Actor model with idle message** — rejected per earlier decision (acd-51l,
  P4). RwLock approach is simpler.
- **Heartbeat/TTL for stale sessions** — filed as acd-a8k (P3). Crashed clients
  that don't send `RM` keep sessions non-closed, preventing idle timer. Not
  blocking for initial implementation.

## Follow-up issues

- **acd-jnd** — make timeout configurable via `config.toml`
- **acd-2zh** — add `SessionStore::has_active_sessions()` to avoid cloning all
  sessions on every tick
- **acd-a8k** — session TTL/heartbeat for crashed clients
