# Decision: Retry and Connection Strategy

**Decided:** 2026-01-18 **Status:** Implemented

## Context

Clients (TUI dashboards, hooks) need reliable connection to the daemon. The
connection flow must handle daemon startup delays, transient failures, and API
fetch errors gracefully.

## Decision

### Connection Retry (Q31)

When a client cannot connect to the daemon:

1. Try connect to daemon socket
2. If fails, auto-start daemon (Q2)
3. Poll for socket file (up to 2s, check every 100ms)
4. Once socket exists, retry connect (3 attempts, 100ms apart)
5. If still fails, show error and exit

```rust
const SOCKET_POLL_TIMEOUT_MS: u64 = 2000;
const SOCKET_POLL_INTERVAL_MS: u64 = 100;
const CONNECTION_RETRIES: u32 = 3;
const CONNECTION_RETRY_DELAY_MS: u64 = 100;
```

Separating daemon startup wait (socket polling) from connection retry is more
reliable than guessing startup time.

### API Error Handling (Q32)

When the Anthropic Usage API fails:

- Show last known data with age: `$1.42 (5m ago)`
- Retry in background every 60 seconds
- After 5 minutes of failures, show warning: `$1.42 (stale)`
- Click/select warning to trigger immediate retry

```rust
const API_RETRY_INTERVAL_SECS: u64 = 60;
const API_STALE_WARNING_SECS: u64 = 300;  // 5 minutes
```

## Rationale

- Socket polling is more reliable than fixed sleep delays
- Stale data with age indicator is better than a blank widget
- Background retry avoids blocking the UI while recovering
- Interactive refresh (click warning) gives users control

## Implementation

[Q31, Q32](../archive/planning/6-open-questions.md)
