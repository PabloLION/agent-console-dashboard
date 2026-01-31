# Concurrency Model

**Created:** 2026-01-31 **Status:** Active **Related Decisions:** D1 in
[2026-01-31-discussion-decisions.md](./2026-01-31-discussion-decisions.md)

## Summary

The daemon uses a single-threaded actor model with an mpsc message queue. All
connections feed into one channel, a single event loop processes messages
sequentially, and the session store is a plain `HashMap` (no `RwLock` needed).

## Architecture

### Message Flow

```text
Connections → mpsc channel → single event loop → process one at a time
     ↓              ↓                ↓
 Unix socket    Bounded queue    Plain HashMap
   (accept)      (messages)       (state store)
     ↓              ↓                ↓
  Hooks/TUIs    Sequential       No locks needed
               processing
```

### Event Multiplexing

```rust
loop {
    tokio::select! {
        conn = listener.accept() => handle_new_connection(conn),
        Some(msg) = rx.recv() => process_message(msg),
        _ = usage_interval.tick() => fetch_usage_if_subscribers(),
        _ = auto_stop_interval.tick() => check_auto_stop(),
    }
}
```

All state mutations go through the single message queue, processed sequentially.

### Store Implementation

```rust
// No Arc, no RwLock — just a plain HashMap
struct DaemonState {
    sessions: HashMap<SessionId, Session>,
    usage: Option<UsageData>,
    subscribers: Vec<ClientConnection>,
}

fn update_session(state: &mut DaemonState, id: SessionId, status: Status) {
    if let Some(session) = state.sessions.get_mut(&id) {
        session.status = status;
        broadcast_update(state, id);
    }
}
```

## Rationale

For our scale (< 50 connections, < 100 messages/sec), sequential processing adds
negligible latency:

- Message processing time: ~10-50 microseconds
- Network latency: ~1-10 milliseconds
- Sequential overhead: < 1% of total latency

### Benefits

- **Eliminates race conditions** — only one thread accesses state
- **No lock contention** — plain HashMap, no RwLock
- **Simpler reasoning** — linear execution, no concurrency bugs
- **Easier debugging** — deterministic message order

### Trade-offs

- No parallel processing (messages processed serially)
- Head-of-line blocking (slow message delays subsequent messages)
- Single CPU core utilization

These are acceptable because message processing is fast (microseconds), no
CPU-intensive operations in handlers, and I/O multiplexing via `tokio::select!`
keeps the system responsive.

## Pattern

This is the **actor model**:

- The daemon's state store is an **actor**
- The mpsc channel is the actor's **mailbox**
- The event loop is the actor's **message processor**

Each message is processed atomically before the next message begins.

## Bounded Channel for Broadcasts

```rust
// Per-subscriber message buffer (independent of max concurrent clients count).
// This bounds backpressure per subscriber, not global connection capacity.
let (tx, rx) = mpsc::channel(100);

if tx.try_send(update).is_err() {
    // Subscriber cannot keep up, disconnect
    disconnect_slow_client(client_id);
}
```

This prevents one slow TUI from blocking broadcasts to all other TUIs.

## Future Scaling

If message volume grows beyond single-thread capacity (unlikely for < 50
connections):

1. Profile first — identify bottleneck
2. Optimize hot path — improve message processing speed
3. Consider sharding — multiple actors for different session groups
4. Only then consider multi-threading

For v0/v1, single-threaded is sufficient.

## References

- [D1: Concurrency Model](./2026-01-31-discussion-decisions.md)
- [Amendment 2: Bounded Channel for Broadcasts](./2026-01-31-decision-amendments.md)
  — per-subscriber bounded channels with backpressure disconnection for slow
  clients
