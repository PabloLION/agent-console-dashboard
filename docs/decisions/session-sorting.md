# Session Sorting

## Decision

Sessions in the TUI are sorted by a lexicographic key with three components:

1. **Status group** (ascending): Attention (0) > Working (1) > Inactive (2) >
   Closed (3)
2. **Priority** (descending): Higher u64 value ranks higher. Default: 0.
3. **Elapsed time** (descending): Longer-running sessions rank higher within
   same group and priority.

Rust implementation uses tuple comparison with `std::cmp::Reverse`:

```rust
(status_group, Reverse(priority), Reverse(elapsed_seconds))
```

## Status Group Derivation

`Status` enum has four variants: Working, Attention, Question, Closed. There is
no "Inactive" variant. Inactive is derived at sort time from
`session.is_inactive(INACTIVE_SESSION_THRESHOLD)` (idle > 3600s and not closed).

Group mapping in `Status::status_group()`:

```text
Attention → 0
Working   → 1
Question  → 2  (same tier as inactive)
Inactive  → 2  (derived from idle time, not status)
Closed    → 3
```

## Priority

- Type: `u64`, default 0
- Stored passively in daemon (same pattern as `working_dir`)
- Set via CLI: `acd set <id> <status> --priority <value>`
- Broadcast: `broadcast_session_change` fires on status OR priority change

## IPC

Priority field added to `IpcCommand` and `SessionSnapshot` with
`#[serde(default)]` for backwards compatibility.

## Related Issues

- acd-wx6: Sort implementation (closed)
- acd-nd1: CLI priority flag (closed)
- acd-50i: TUI priority interface (open, P4)
- acd-2jp: CLI tree redesign (open, P2)
