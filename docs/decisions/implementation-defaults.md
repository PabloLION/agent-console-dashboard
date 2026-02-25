# Decision: Implementation Defaults

**Status:** Implemented

This file groups simple implementation decisions that have straightforward
rationale. Each is self-contained but did not warrant a separate file.

## Signal Handling (Q26)

**Decided:** 2026-01-17

| Signal  | Behavior                                     |
| ------- | -------------------------------------------- |
| SIGTERM | Graceful shutdown (same as `acd stop`)       |
| SIGINT  | Same as SIGTERM (for foreground/debug mode)  |
| SIGHUP  | Reload configuration (see config-and-reload) |

## Invalid Messages (Q30)

**Decided:** 2026-01-18

Daemon returns error response (`{"error": "invalid JSON: <parse error>"}`), logs
warning, and keeps connection open for retry.

## Socket Permissions (Q33)

**Decided:** 2026-01-18

User-only `0600`. Single-user tool with no need for shared access.

## Input Validation (Q35)

**Decided:** 2026-01-18

Basic length limits only. Input comes from Claude Code (trusted source).

| Field          | Max length | Validation         |
| -------------- | ---------- | ------------------ |
| `display_name` | 64 chars   | Truncate if longer |
| `cwd`          | 256 chars  | Truncate if longer |
| `session_id`   | 128 chars  | Truncate if longer |

## Soft Limits (Q36, Q37)

**Decided:** 2026-01-18

Warning at 50 sessions and 50 dashboards. No hard limit. Memory per session is
~1KB.

## History Depth (Q38)

**Decided:** 2026-01-18

Combined limit per session: last 200 transitions AND within 24 hours (whichever
hits first).

## Unicode Support (Q43)

**Decided:** 2026-01-18

Full unicode support. Display as-is, trust the terminal to render. Ratatui
handles unicode well.

## Shell Completions (Q45)

**Decided:** 2026-01-18

Built-in via `clap_complete` crate: `acd completions bash|zsh|fish`.

## Logging (Q47)

**Decided:** 2026-01-18

File logging at `~/.local/share/agent-console/logs/`. Plain text format. Levels
configurable via `RUST_LOG` env var or config file. Falls back to stderr if log
file is not writable (Q66).

## Testing Strategy (Q48)

**Decided:** 2026-01-18

| Level       | What                          | How                        |
| ----------- | ----------------------------- | -------------------------- |
| Unit        | Data structures, parsing      | Standard Rust tests        |
| Integration | Daemon + client communication | Spawn daemon, mock clients |
| End-to-end  | Full workflow with mock hooks | `acd test-client` command  |

## Feature Flags (Q49)

**Decided:** 2026-01-18

No feature flags for v0. Everything always included. Revisit if binary size
becomes a concern.

## MSRV (Q53)

**Decided:** 2026-01-18 (updated 2026-02-24)

The project pins to a specific Rust version via `rust-toolchain.toml` for CI
reproducibility. Pinning ensures identical `rustfmt` output across all machines
(local and CI), preventing spurious formatting diffs from toolchain upgrades.

The pinned version and component list (`rustfmt`, `clippy`) are recorded in
`rust-toolchain.toml`. Update the pin deliberately when upgrading.

## Daemonization (Q56)

**Decided:** 2026-01-18

`--daemonize` flag via `daemonize` crate. `acd daemon` runs foreground (dev),
`acd daemon -d` forks and detaches (normal usage).

## PID File (Q57)

**Decided:** 2026-01-18

Both PID file (`~/.local/share/agent-console/daemon.pid`) and socket check.
Stale PID file (process dead) is cleaned up to allow restart.

## Multiple Daemon Instances (Q58)

**Decided:** 2026-01-18

Detect and refuse. If daemon already running: "Daemon already running (PID
1234)".

## Health Check (Q59)

**Decided:** 2026-01-18

`acd status` sends PING, expects PONG. Shows running/stopped, PID, uptime,
connected dashboards count.

## Protocol Versioning (Q60, Q61, Q62)

**Decided:** 2026-01-18

Newline-delimited JSON. Version field in every message. Lenient compatibility:
ignore unknown fields, use defaults for missing.

## Config Edge Cases (Q63, Q65)

**Decided:** 2026-01-18

Unknown config keys: warn but continue. First run: work without config file
(zero friction).

## Environment Variable Overrides (Q64)

**Decided:** 2026-01-18

All settings overridable via env vars. Priority: env var > config file >
default.

## Error Recovery (Q66, Q67, Q68)

**Decided:** 2026-01-18

- **Disk full (Q66):** Fall back to stderr, continue running
- **Corrupt history (Q67):** Backup to `history.json.corrupt.{timestamp}`, start
  fresh
- **Socket path not writable (Q68):** Create parent dirs, exit with clear error
  if still fails

## Claude Code Version Support (Q70)

**Decided:** 2026-01-18

Support latest only. Document minimum: "Requires Claude Code 2.0.76+".

## Usage Fetch Refresh (Q86)

**Decided:** 2026-01-22

Default 180s (3 min). Minimum 1s, maximum 3600s. Invalid values produce warning
and fall back to default.

## Clock Skew (Q92)

**Decided:** 2026-01-22

Show actual calculated value. Weird percentages (-5% or 150%) signal to user
their clock is wrong. More transparent than silent clamping.

---

[All questions](../archive/planning/6-open-questions.md) |
[Original analysis](../archive/planning/7-decisions.md)
