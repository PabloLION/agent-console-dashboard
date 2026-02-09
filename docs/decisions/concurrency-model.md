# Decision: Concurrency Model

**Decided:** 2026-01-31 **Status:** Implemented

## Context

The original backend architecture (Q1) left the concurrency model implicit,
assuming `tokio::spawn` per connection with `Arc<RwLock<HashMap>>` for shared
state. The epic quality review identified this as a gap needing explicit design.

## Decision

The daemon uses a single-threaded actor model. All connections feed into one
mpsc channel. A single event loop processes messages sequentially and mutates a
plain `HashMap` (no `RwLock`).

```text
Connections -> mpsc channel -> single event loop -> process one message at a time
```

`tokio::select!` multiplexes I/O (accept connections, read from sockets,
timers), but all state mutations go through one queue processed sequentially.

## Rationale

- At our scale (< 50 connections, < 100 messages/sec), sequential processing
  adds negligible latency
- Eliminates `RwLock` entirely, replacing it with a plain `HashMap`
- Eliminates all race conditions on store access
- Simpler to reason about: one queue, one processor

## Alternatives Considered

- **`tokio::spawn` per connection + `Arc<RwLock<HashMap>>`**: the original
  implicit assumption. Rejected because RwLock adds complexity and potential for
  subtle race conditions with no performance benefit at our scale.

## Amendments

This decision itself is Amendment 2 (2026-01-31), replacing the implicit RwLock
assumption from Q1. See also [D1](../archive/planning/discussion-decisions.md)
and [Amendment 2](../archive/planning/decision-amendments.md).

## Implementation

Tracked in E001 (daemon core) and E003 (IPC protocol).

[D1](../archive/planning/discussion-decisions.md) |
[Amendment 2](../archive/planning/decision-amendments.md) |
[Original Q1](../archive/planning/7-decisions.md)
