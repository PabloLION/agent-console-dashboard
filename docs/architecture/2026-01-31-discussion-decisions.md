# Architecture Discussion Decisions

**Date:** 2026-01-31 **Context:** Epic quality review and architectural
clarification session

These decisions were made during the epic review discussion to resolve gaps
identified by the BMAD architect review. They refine and extend the original
decisions in `docs/plans/7-decisions.md`.

## D1: Concurrency Model — Single-Threaded Actor

**Decision:** Single-threaded tokio runtime with mpsc message queue.

**Replaces:** Implicit assumption of `tokio::spawn` per connection with
`Arc<RwLock<HashMap>>`.

**Rationale:**

- For our scale (< 50 connections, < 100 messages/sec), sequential processing
  adds negligible latency
- Eliminates `RwLock` entirely — plain `HashMap` for store
- Eliminates all race conditions on store access
- Simpler to reason about: one queue, one processor

**Pattern:**

```text
Connections → mpsc channel → single event loop → process one message at a time
```

`tokio::select!` multiplexes I/O (accept connections, read from sockets,
timers), but all state mutations go through one queue processed sequentially.
This is the actor model — the daemon's store is an actor with a single mailbox.

## D2: Protocol Format — JSON Lines over Unix Socket

**Decision:** JSON Lines (one JSON object per `\n`-delimited line) over Unix
socket. Transport and serialization are independent layers.

**Extends:** Q15 (IPC Protocol Format)

**Rationale:**

- JSON serializers escape `\n` inside strings, so no framing ambiguity
- Unix socket is Linux/macOS only (Q23 defers Windows)
- Future Windows support: named pipes with same JSON Lines format
- Future v1+: could swap transport to SQLite or TCP without changing wire format

**Layer separation:**

| Layer         | v0/v1       | Future                         |
| ------------- | ----------- | ------------------------------ |
| Serialization | JSON Lines  | JSON Lines                     |
| Transport     | Unix socket | Named pipes (Win), TCP, SQLite |

## D3: Widget Data Flow — Fully Centralized via Daemon

**Decision:** Daemon is the single source of truth for ALL data. TUI only talks
to daemon. Widgets only read from `WidgetContext`.

**Replaces:** Mixed pattern where most widgets used WidgetContext but api-usage
widget called `claude_usage::get_usage()` directly.

**Rationale:**

- With N TUIs, decentralized fetch = N API calls per interval (wasteful)
- Risk of rate limiting from Anthropic API
- Daemon fetches once, broadcasts to all subscribers
- Consistent architecture: no special cases per widget

**Data flow:**

```text
claude-usage crate → daemon (fetches every 3 min) → broadcast to TUIs
hooks (JSON stdin) → daemon (session state) → broadcast to TUIs
TUI receives all data → populates WidgetContext → passes to widgets
```

**Fetch behavior:** Daemon only fetches usage when ≥1 TUI is subscribed. No
audience = no API calls. Decision finalized for v0.

## D4: Usage Fetch Interval — 3 Minutes

**Decision:** Daemon fetches API usage every 3 minutes.

**Rationale:**

- 5-hour window = 300 minutes
- 1% of 300 minutes = 3 minutes
- Fetching every 3 minutes aligns with 1% accuracy granularity
- This means the displayed percentage can be at most 1% stale

**Configurable:** Yes, via `[daemon] usage_fetch_interval = "3m"` in config.

**Known concern:** Timer alignment — the fetch interval and the API's tracking
window may not align perfectly due to rounding. When we round displayed
percentages, they may fall into an adjacent 3-minute bucket. Decision finalized
for v0.

## D5: Auto-Stop Timeout — 60 Minutes (Configurable)

**Decision:** Auto-stop after 60 minutes idle (was 30 minutes).

**Amends:** Q25 in `docs/plans/7-decisions.md`

**Rationale:**

- 60 minutes is more forgiving for intermittent usage patterns
- Combined with auto-start, eliminates concern about rapid socket create/delete
  cycles (debounce effect)
- Configurable: users who want aggressive cleanup can set lower values

**Constants updated:**

```rust
const AUTO_STOP_CHECK_INTERVAL_SECS: u64 = 300;   // 5 minutes (unchanged)
const AUTO_STOP_IDLE_THRESHOLD_SECS: u64 = 3600;  // 60 minutes (was 1800)
```

## D6: Socket Cleanup — Clean on Shutdown

**Decision:** Remove socket file on daemon shutdown (auto-stop, SIGTERM,
`acd stop`). No stale socket detection needed on startup.

**Confirms:** Existing S001.02 AC

**Rationale:**

- Auto-stop at 60 minutes means socket won't be rapidly created/deleted
- Auto-start handles the "daemon not running" case transparently
- Stale socket detection adds complexity for a scenario that shouldn't occur
  with proper shutdown

## D7: Error Propagation — Daemon to TUI

**Decision:** Daemon errors propagate to all connected TUI dashboards. TUI
displays errors in bottom-right corner. Hooks are fire-and-forget.

**Error flow:**

| Source                  | Handling                                             |
| ----------------------- | ---------------------------------------------------- |
| Daemon internal error   | Log + broadcast error to all TUIs                    |
| Hook connection failure | Silent fail; daemon auto-starts on next attempt      |
| TUI receives error      | Display in bottom-right status area                  |
| Usage fetch failure     | Show "unavailable" in widget, retry on next interval |

## D8: Session Identification — JSON stdin session_id

**Confirms:** Q16 in `docs/plans/7-decisions.md`

Claude Code exposes `session_id` in JSON stdin to ALL hook types (Stop,
UserPromptSubmit, Notification, PreToolUse, SessionStart, SessionEnd, etc.).
This is the primary session identifier.

Hook scripts parse stdin JSON to extract `session_id`:

```bash
INPUT=$(cat)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id')
agent-console set "$SESSION_ID" working
```

The `basename "$PWD"` pattern in some epic examples is stale and must be
replaced with stdin JSON parsing.
