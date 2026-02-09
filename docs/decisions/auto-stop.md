# Decision: Auto-Stop

**Decided:** 2026-01-17 (amended 2026-01-31) **Status:** Implemented

## Context

The daemon should stop automatically when idle to avoid wasting resources, but
the threshold must be forgiving enough to avoid annoying stop/start cycles
during intermittent usage.

## Decision

The daemon auto-stops after **60 minutes idle** (configurable). Auto-stop
triggers when all three conditions are met simultaneously for the threshold
duration:

1. No dashboards connected
2. No active sessions
3. Condition persists for idle threshold

```rust
const AUTO_STOP_CHECK_INTERVAL_SECS: u64 = 300;   // 5 minutes
const AUTO_STOP_IDLE_THRESHOLD_SECS: u64 = 3600;  // 60 minutes
```

Configurable via `[daemon] idle_timeout = "60m"` in config.

## Rationale

- 60 minutes is forgiving for intermittent usage patterns
- Combined with auto-start, eliminates concern about rapid socket create/delete
  cycles (debounce effect)
- Resource usage is near zero while idle (kernel timer, wakes every 5 minutes
  for ~1ms of CPU)

## Alternatives Considered

- **No auto-stop**: wastes resources indefinitely
- **30 minutes** (original Q25 value): too aggressive for intermittent usage
- **User-initiated only**: requires manual cleanup

## Amendments

Amendment 1 (2026-01-31) changed the threshold from 30 minutes to 60 minutes.
See [D5](../archive/planning/discussion-decisions.md) and
[Amendment 1](../archive/planning/decision-amendments.md).

## Implementation

Part of the daemon shutdown system (Q25) which includes `acd stop`, SIGTERM
handling, and auto-stop. Socket cleanup happens on all shutdown paths.

[Q25 in 7-decisions](../archive/planning/7-decisions.md) |
[D5](../archive/planning/discussion-decisions.md)
