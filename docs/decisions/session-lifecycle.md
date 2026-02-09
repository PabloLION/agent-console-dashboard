# Decision: Session Lifecycle

**Decided:** 2026-01-17 **Status:** Implemented

## Context

Sessions move through multiple states during their lifetime. Decisions were
needed on what statuses to track, how to detect session closure, and what
happens to closed sessions.

## Decision

### Status Types (Q4)

Four statuses as a C-like enum (all unit variants):

```rust
enum Status {
    Working,    // Agent is processing
    Attention,  // Agent needs user attention
    Question,   // Agent asked a question (AskUserQuestion)
    Closed,     // Session ended
}
```

Additional statuses (Error, Paused, Rate Limited) were deferred to keep v0
simple. The enum can be extended without breaking changes.

### Closed Detection (Q18)

SessionEnd hook only (no timeout). Claude Code fires `SessionEnd` when a session
ends cleanly. If Claude Code crashes without firing the hook, the session stays
visible until the user manually removes it.

### Closed Session Fate (Q5)

Closed sessions move to a dedicated history space (not shown on the main
dashboard). History is persisted to `~/.config/agent-console/history.json` with
no automatic removal. Access via `acd history` or TUI keyboard shortcut.

## Lifecycle Flow

```text
1. Session starts          -> Status: Working
2. Session runs            -> Working <-> Attention <-> Question
3. Session ends (hook)     -> Status: Closed, moves to history
4. Closed session fate:
   - User resurrects       -> Status back to Working
   - User manually removes -> Removed from history
```

## Rationale

- No timeout for closure avoids incorrectly marking active-but-idle sessions
- Dedicated history keeps the main dashboard clean
- Count-based cleanup (`max_closed_sessions = 20`) prevents unbounded growth
- Resurrection uses working directory only (Claude Code has its own session
  picker)

## Implementation

Duration tracking for all statuses (Q51): timestamps record when each status
began, enabling "worked 45min, waited 5min" displays.

[Q4, Q5, Q18](../archive/planning/6-open-questions.md) |
[Q5 in 7-decisions](../archive/planning/7-decisions.md)
