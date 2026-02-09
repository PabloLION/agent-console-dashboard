# Error Handling Strategy

**Created:** 2026-01-31 **Status:** Active **Related Decisions:**
[Error Propagation](../decisions/error-propagation.md)

## Summary

Errors propagate from daemon to all connected TUI dashboards via broadcast
messages. TUIs display errors in a bottom-right status area. Hooks are
fire-and-forget — if the daemon is down, they fail silently (daemon auto-starts
on next attempt).

## Error Flow by Component

### Daemon Internal Errors

| Source               | Handling                                                  |
| -------------------- | --------------------------------------------------------- |
| Session store error  | Log + broadcast to all TUIs                               |
| Config parse failure | Log + keep old config, notify TUIs                        |
| Usage fetch failure  | Log + broadcast "unavailable" status, retry next interval |

### Hook Errors

| Scenario             | Handling                                                |
| -------------------- | ------------------------------------------------------- |
| Daemon not running   | Hook fails silently, daemon auto-starts on next attempt |
| Socket write failure | Hook exits (ephemeral process, no retry)                |
| Invalid hook payload | Daemon logs error, ignores message                      |

Hooks are **fire-and-forget** — ephemeral processes with no retry logic.

### TUI Connection Errors

| Scenario                 | Handling                                                   |
| ------------------------ | ---------------------------------------------------------- |
| Daemon disconnects       | Show "disconnected" in status, auto-reconnect with backoff |
| Initial connection fails | Trigger daemon auto-start, retry connection                |
| Slow subscriber          | Daemon disconnects slow TUI (bounded channel full)         |

### Usage Fetch Errors

| Scenario             | Handling                                  |
| -------------------- | ----------------------------------------- |
| API request fails    | Log, show "unavailable" in widget         |
| Authentication error | Log, broadcast to TUIs                    |
| Rate limit hit       | Log warning, retry on next 3-min interval |

## Error Categories

```rust
enum DaemonError {
    Network(String),    // Socket, connection issues
    State(String),      // Store corruption, invalid session state
    Config(String),     // Config parse/reload failures
    Hook(String),       // Invalid hook messages
    UsageApi(String),   // API fetch failures
}
```

## TUI Error Display

Errors appear in the bottom-right status area:

```text
┌─ Agent Console Dashboard ──────────────────────────────┐
│  ● proj-a      Working      ~/projects/proj-a          │
│  ○ proj-b      Attention    ~/projects/proj-b  2m34s   │
│  Quota: 5h 8% | 7d 77% | resets 2h 15m                │
│  [j/k] Navigate  [Enter] Details       [⚠ Daemon error]│
└────────────────────────────────────────────────────────┘
```

- **Transient errors** — show for 5 seconds, then fade
- **Persistent errors** — remain visible until resolved
- **Multiple errors** — show most recent, cycle on key press

## Retry Strategy

| Component         | Strategy                                              |
| ----------------- | ----------------------------------------------------- |
| Hooks             | No retry (ephemeral, exit on failure)                 |
| TUI connection    | Exponential backoff: 100ms, 200ms, 400ms, ..., max 5s |
| Usage fetch       | Fixed interval: retry every 3 minutes                 |
| Daemon auto-start | Single attempt, report error to caller                |

## References

- [Error Propagation Decision](../decisions/error-propagation.md)
- [D7](../archive/planning/discussion-decisions.md) |
  [Q24](../archive/planning/7-decisions.md) (archived sources)
