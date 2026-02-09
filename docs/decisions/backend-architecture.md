# Decision: Backend Architecture

**Decided:** 2026-01-17 **Status:** Implemented

## Context

The project needed a state management approach for tracking Claude Code session
data across multiple terminal panes. Three architectures were evaluated: an
embedded SQLite database, POSIX shared memory, and a long-running daemon with
Unix socket IPC.

## Decision

A single daemon process manages all state in memory as a plain `HashMap`. Hooks
and TUI dashboards communicate with it over a Unix socket.

## Rationale

| Aspect         | SQLite           | Shared Memory   | Daemon       |
| -------------- | ---------------- | --------------- | ------------ |
| Binary size    | +1-2MB           | +0              | +0           |
| Safety         | Safe Rust        | Unsafe required | Safe Rust    |
| Persistence    | Built-in         | None            | None         |
| Real-time      | Polling (~100ms) | Yes             | Push (<10ms) |
| Complexity     | Medium           | High            | Medium       |
| Crash recovery | Automatic        | State lost      | State lost   |

The daemon fits because:

- Minimal footprint: one socket file, no database file
- Push model eliminates polling for real-time updates
- Sessions are transient; persistence is unnecessary
- Safe Rust throughout (no `unsafe` code)
- Simple data model (HashMap in memory, no SQL schema)

## Alternatives Considered

- **Shared Memory** rejected for requiring `unsafe` Rust, platform-specific
  APIs, complex synchronization with mutexes, and restriction to fixed-size POD
  types
- **SQLite** rejected for adding 1-2MB binary size, requiring polling for
  updates, schema/migrations overhead, and unnecessary persistence for volatile
  state

## Amendments

Amendment 2 (2026-01-31) refined the concurrency model within the daemon: the
original implicit assumption of `tokio::spawn` per connection with
`Arc<RwLock<HashMap>>` was replaced by a single-threaded actor model with mpsc
queue and plain `HashMap`. See [concurrency-model.md](concurrency-model.md).

## Crash Handling

Daemon crash means state is lost, which is acceptable. Hooks re-register on the
next event (daemon auto-starts per Q2), and sessions refresh quickly through
normal user interaction.

## Implementation

[Original analysis](../archive/planning/7-decisions.md) |
[Q1](../archive/planning/6-open-questions.md)
