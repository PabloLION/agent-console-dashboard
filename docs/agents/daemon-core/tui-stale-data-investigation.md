# TUI Stale Data Investigation

**Issue**: After the user is AFK for an extended period, the TUI stops displaying new updates from the daemon. Sessions appear frozen/stale.

**Research Date**: 2026-02-17
**Working Directory**: /tmp/acd-wt-daemon-core-arsz

---

## Summary

The TUI likely stops receiving updates due to **Unix socket connection failure without automatic reconnection**. The TUI establishes a single connection to the daemon at startup via the SUB subscription, but there is no heartbeat/keep-alive mechanism and no automatic reconnection logic when the connection drops.

**Root cause**: The TUI's subscription task (spawned in `event_loop()` at line 404 in `app/mod.rs`) runs once at startup and relies on a persistent Unix socket connection. If this connection is broken during idle periods (by OS-level socket timeouts, macOS sleep/wake, network stack cleanup, or daemon restarts), the subscription task silently exits and the TUI continues running with stale data. The main event loop has no mechanism to detect or recover from subscription failure.

---

## Connection Lifecycle

### Initial Connection Flow

1. **TUI startup** (`app/mod.rs:394-408`):
   - Main event loop starts
   - Creates mpsc channel `(update_tx, update_rx)` for daemon messages
   - Spawns background task running `subscribe_to_daemon()`
   - Task continues independently; errors are logged but not propagated

2. **Subscription establishment** (`subscription.rs:25-105`):
   - Opens first connection → sends `LIST` command → receives all sessions
   - **Opens second connection** (line 66) → sends `SUB` command
   - Enters infinite loop reading JSON lines from socket (lines 89-104)
   - On read success: parses and sends via `update_tx`
   - On `bytes == 0` (EOF): breaks loop and task exits silently
   - On I/O error: returns error (logged by spawn wrapper at line 405-407)

3. **Daemon SUB handler** (`daemon/handlers/mod.rs:173-315`):
   - Acknowledges subscription
   - Subscribes to two tokio broadcast channels:
     - `session_rx` from SessionStore (line 182)
     - `usage_sub` from UsageFetcher (line 185)
   - Runs `loop { tokio::select! { ... } }` forwarding broadcasts to client
   - Exits when write fails (client disconnected) or broadcast channel closes

### Normal Operation

- Daemon broadcasts `SessionUpdate` on every status/priority change (`store/mod.rs:114-135`)
- Broadcast uses tokio's `broadcast::channel` (capacity 256, line 22)
- SUB handler forwards broadcasts to client via Unix socket write
- TUI reads messages from `update_rx` channel (`app/mod.rs:412-419`)
- Updates applied via `apply_update()` method

### Failure Points During Idle

1. **Unix socket idle timeout** (macOS):
   - macOS may close idle Unix domain sockets after prolonged inactivity (no definitive timeout documented)
   - When socket closes, TUI's `read_line()` returns 0 bytes (EOF)
   - Subscription task breaks loop and exits (line 93)

2. **macOS sleep/wake**:
   - System sleep may invalidate socket file descriptors
   - Daemon continues running, but TUI's socket connection becomes stale
   - Next `read_line()` returns error or EOF

3. **Daemon restart**:
   - If daemon restarts (manual or auto-stop idle timeout), socket path is recreated
   - TUI's existing connection points to old socket
   - Connection fails with "connection reset" or EOF

4. **Broadcast channel overflow**:
   - If TUI doesn't read from socket fast enough, daemon's broadcast channel lags
   - Daemon logs "Subscriber lagged, missed N messages" (line 244, 269)
   - Daemon sends `IpcNotification::warn("lagged N")` to TUI
   - **TUI does not process this warning** — `parse_daemon_line()` returns `None` for warn messages (line 131-136)
   - Connection stays alive but data may be incomplete

---

## No Heartbeat or Keep-Alive Mechanism

**Searched patterns**: `timeout`, `keepalive`, `reconnect` across all `.rs` files.

**Findings**:
- No socket-level keep-alive configuration (no `SO_KEEPALIVE`, `TCP_KEEPALIVE_*`)
- No application-level heartbeat or ping/pong messages
- No periodic dummy writes to keep connection active
- Daemon has idle timeout for auto-shutdown (`idle_timeout` config, default 60m), but this is for daemon lifecycle, not connection health

**No TCP keep-alive**: Unix domain sockets (used here) do not support TCP keep-alive options because they are IPC, not network sockets.

---

## No Automatic Reconnection Logic

**TUI** (`app/mod.rs:404-408`):
```rust
tokio::spawn(async move {
    if let Err(e) = subscribe_to_daemon(&socket_path, update_tx).await {
        tracing::warn!("daemon subscription failed: {}", e);
    }
});
```

- Subscription task runs **once**
- On error, logs warning and exits
- Main event loop (`app/mod.rs:410-513`) has no logic to:
  - Detect that subscription task has exited
  - Restart subscription task
  - Alert user to connection loss

**Result**: After subscription task exits, TUI continues rendering with stale session data. User input still works (keyboard/mouse), elapsed times continue incrementing, but no new session updates arrive.

---

## Why This Manifests After Idle Period

1. **Socket is active during normal use**:
   - Claude Code hooks fire frequently during active session
   - Daemon broadcasts updates
   - Data flows through socket, preventing OS-level idle cleanup

2. **Socket becomes idle when user is AFK**:
   - No new hook events
   - No session status changes
   - Socket has no traffic for extended period
   - OS may clean up idle resources

3. **Daemon's auto-stop idle timeout** (default 60 minutes):
   - If daemon has no active sessions for 60 minutes, it auto-stops
   - TUI's connection becomes invalid
   - TUI continues running with stale data

---

## Tokio Unix Socket Behavior

**Tokio's `UnixStream`** does not have default timeouts:
- `read_line()` blocks indefinitely until data arrives or connection closes
- No built-in idle timeout
- Connection closure (daemon exit, OS cleanup) returns EOF (0 bytes)

**Code evidence** (`subscription.rs:91`):
```rust
let bytes = reader.read_line(&mut line).await?;
if bytes == 0 {
    break;
}
```

When connection breaks, `read_line()` returns `Ok(0)`, task exits silently.

---

## Recommendation

**Primary fix**: Add automatic reconnection logic to the TUI.

### Option A: Retry Loop in Subscription Task (Recommended)

Wrap `subscribe_to_daemon()` in a retry loop with exponential backoff:

```rust
tokio::spawn(async move {
    let mut backoff = Duration::from_millis(100);
    loop {
        match subscribe_to_daemon(&socket_path, update_tx.clone()).await {
            Ok(()) => {
                tracing::info!("Daemon subscription ended, reconnecting...");
            }
            Err(e) => {
                tracing::warn!("Daemon subscription failed: {}, retrying in {:?}", e, backoff);
            }
        }
        sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(30));
    }
});
```

**Pros**:
- Simple, minimal code change
- Survives daemon restarts
- Handles all connection failure modes

**Cons**:
- No user feedback when disconnected (could add status indicator)
- Reconnection delay introduces brief staleness

### Option B: Heartbeat/Keep-Alive Messages

Add periodic ping/pong between daemon and TUI:

- Daemon sends `IpcNotification { type: "ping", ... }` every 30 seconds to SUB clients
- TUI expects ping within timeout window (e.g., 60 seconds)
- If ping is late, TUI assumes connection is stale and reconnects

**Pros**:
- Proactive detection of broken connections
- Can display "reconnecting" status to user

**Cons**:
- More complex (two-way protocol change)
- Increased traffic during idle periods
- Requires daemon changes

### Option C: Health Check from Main Event Loop

Main event loop monitors `update_rx` channel and reconnects if no messages arrive for N seconds:

```rust
let mut last_update = Instant::now();
loop {
    // Check if subscription is stale
    if last_update.elapsed() > Duration::from_secs(120) {
        // Restart subscription task
    }

    while let Ok(msg) = update_rx.try_recv() {
        last_update = Instant::now();
        // apply update
    }
}
```

**Pros**:
- Main event loop is aware of connection health
- Can show status indicator

**Cons**:
- False positives during legitimate idle periods (no session changes)
- Requires tracking last-update timestamp
- More invasive change to event loop

---

## Chosen Approach

**Recommendation: Option A (Retry Loop)** for initial fix.

1. Minimal code change (wrap spawn in retry loop)
2. Handles all failure modes (daemon restart, socket timeout, sleep/wake)
3. No protocol changes required
4. Can add Option B (heartbeat) later for proactive detection if needed

**Follow-up improvements**:
- Add connection status indicator in TUI footer
- Log reconnection events for debugging
- Expose reconnection stats in `STATUS` command

---

## Related Code Paths

| File | Lines | Purpose |
|------|-------|---------|
| `tui/subscription.rs` | 25-105 | Connection + SUB loop |
| `tui/app/mod.rs` | 394-408 | Spawn subscription task |
| `tui/app/mod.rs` | 410-419 | Drain updates in event loop |
| `daemon/handlers/mod.rs` | 173-315 | SUB command handler |
| `daemon/store/mod.rs` | 114-135 | Broadcast session changes |
| `client/connection/mod.rs` | 215-276 | `connect_with_lazy_start` logic |

---

## Open Questions

1. **Is there a specific macOS idle timeout for Unix sockets?**
   - Not found in documentation
   - Empirical testing needed (leave TUI idle for 1h, 2h, 4h)

2. **Does daemon auto-stop affect TUI subscriptions?**
   - Yes — daemon closes socket on shutdown
   - TUI's connection becomes invalid
   - Next hook will lazy-start daemon, but TUI won't reconnect

3. **Should we add a `PING` IPC command for health checks?**
   - Not necessary for initial fix
   - Could be added later for observability

---

## Test Plan (After Fix)

1. **Short idle test**: Leave TUI idle for 5 minutes, trigger hook, verify TUI updates
2. **Long idle test**: Leave TUI idle for 90 minutes (beyond daemon auto-stop), trigger hook, verify TUI reconnects
3. **macOS sleep test**: Start TUI, sleep macOS, wake, trigger hook, verify TUI updates
4. **Daemon restart test**: Start TUI, manually stop daemon, trigger hook (lazy-start), verify TUI reconnects
5. **Concurrent reconnect test**: Start 10 TUIs, stop daemon, trigger hooks, verify all TUIs reconnect

---

## Conclusion

The TUI's stale data problem is caused by **lack of automatic reconnection** when the Unix socket connection to the daemon fails. The subscription task exits silently on connection loss, leaving the TUI rendering stale session data indefinitely.

**Fix**: Wrap the subscription task spawn in a retry loop with exponential backoff. This ensures the TUI reconnects automatically after daemon restarts, socket timeouts, or system sleep/wake events.

**No protocol changes needed** — this is purely a TUI-side improvement.
