# Decision: Socket Location and Cleanup

**Decided:** 2026-01-17 (confirmed 2026-01-31) **Status:** Implemented

## Context

Unix sockets need a file path as an address. The socket file stores nothing (0
bytes); it is just an IPC endpoint. macOS and Linux have different conventions
for runtime file locations.

## Decision

Platform-specific socket locations with cleanup on all shutdown paths.

| Platform | Socket Location             | Rationale                      |
| -------- | --------------------------- | ------------------------------ |
| Linux    | `$XDG_RUNTIME_DIR/acd.sock` | XDG standard for runtime files |
| macOS    | `$TMPDIR/acd.sock`          | Apple's user-specific temp dir |

The socket file is removed on daemon shutdown (auto-stop, SIGTERM, `acd stop`).
No stale socket detection is needed on startup because proper shutdown always
cleans up, and the 60-minute auto-stop prevents rapid create/delete cycles.

## Rationale

macOS does not follow XDG and likely never will. Apple has its own conventions:

| Purpose | Linux (XDG)        | macOS (Apple)            |
| ------- | ------------------ | ------------------------ |
| Config  | `~/.config/`       | `~/Library/Preferences/` |
| Runtime | `$XDG_RUNTIME_DIR` | `$TMPDIR`                |

Platform-appropriate locations are used rather than forcing Linux conventions on
macOS.

## Alternatives Considered

- **`/tmp/acd.sock`**: simple but shared by all users (security concern)
- **`~/.config/.../acd.sock`**: wrong place for runtime files
- **XDG everywhere**: macOS does not support XDG

## Amendments

D6 (2026-01-31) confirmed socket cleanup on shutdown and decided against stale
socket detection, relying on auto-stop + auto-start instead.

## Implementation

Socket permissions are set to `0600` (user-only, Q33). If the socket path is not
writable, the daemon creates parent directories first, then exits with a clear
error if creation still fails (Q68).

[Q14 in 7-decisions](../archive/planning/7-decisions.md) |
[D6](../archive/planning/discussion-decisions.md) |
[Q68](../archive/planning/6-open-questions.md)
