# History Display Format

**Decided:** 2026-02-24 **Status:** Implemented

## Decision

Status history shows per-state duration (e.g., `5m32s working → attention`), not
wall-clock timestamps (e.g., `3 minutes ago`).

Duration is derived by diffing consecutive `at_secs` values from `StatusChange`
history entries.

## Rationale

Duration emphasizes patterns over timeline. When diagnosing a session, the
useful question is "how long was it stuck?" not "when did it change?". Duration
answers that directly; "time ago" requires mental arithmetic to reconstruct
dwell time.

Duration is also more stable — it does not change as time passes, so the display
does not drift while the user is looking at it.

## Alternatives Considered

- **Wall-clock timestamps** (`3 minutes ago`, `14:32:01`): Shows when each
  transition happened. Rejected because reconstructing dwell time requires
  subtracting adjacent entries, which is error-prone to do mentally.
- **Both duration and timestamp**: Shows redundant information. Rejected to keep
  the panel compact.
