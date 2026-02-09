# Decision: Error Propagation

**Decided:** 2026-01-17 (refined 2026-01-31) **Status:** Implemented

## Context

Errors can originate from the daemon, from hooks, or from external API calls.
Each source needs a clear propagation path so users are informed without
disrupting Claude Code's operation.

## Decision

Daemon errors propagate to all connected TUI dashboards. The TUI displays errors
in the bottom-right status area. Hooks are fire-and-forget from the daemon's
perspective.

| Source                  | Handling                                             |
| ----------------------- | ---------------------------------------------------- |
| Daemon internal error   | Log + broadcast error to all TUIs                    |
| Hook connection failure | Silent fail; daemon auto-starts on next attempt      |
| TUI receives error      | Display in bottom-right status area                  |
| Usage fetch failure     | Show "unavailable" in widget, retry on next interval |

### Crash Recovery (Q24)

When the daemon crashes:

```text
1. Daemon crashes
2. Dashboard detects disconnect -> shows "?" indicator
3. User continues working (or notices error)
4. Next hook fires -> daemon auto-starts (Q2)
5. Sessions re-register via hooks
6. Dashboard reconnects -> normal display resumes
```

Auto-restart watchdog was deferred to v2+ because coordinating which of the many
dashboard processes should own the watchdog adds complexity for minimal benefit.

## Rationale

- Errors are broadcast to dashboards, never sent to Claude (avoids wasting
  context space)
- Natural recovery through auto-start is sufficient for v0/v1
- Usage fetch failures show stale data with age indicators rather than blank
  widgets

## Implementation

[Q24](../archive/planning/7-decisions.md) |
[D7](../archive/planning/discussion-decisions.md) |
[Q32](../archive/planning/6-open-questions.md)
