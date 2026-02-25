# Decision: IPC Protocol

**Decided:** 2026-01-17 (refined 2026-01-31) **Status:** Implemented

## Context

The daemon and its clients (hooks, TUI dashboards) need a communication format
over the Unix socket. The protocol needed to be debuggable, simple to implement,
and performant enough for low-frequency messages (few per minute).

## Decision

JSON Lines (one JSON object per `\n`-delimited line) over Unix socket. Transport
and serialization are independent layers.

| Layer         | v0/v1       | Future                         |
| ------------- | ----------- | ------------------------------ |
| Serialization | JSON Lines  | JSON Lines                     |
| Transport     | Unix socket | Named pipes (Win), TCP, SQLite |

Every message includes a version field: `{"version": 1, "cmd": "...", ...}`.
Unknown fields are ignored (forward compatible); missing fields use defaults
(backward compatible).

## Rationale

| Format      | Speed  | Size        | Readability            |
| ----------- | ------ | ----------- | ---------------------- |
| JSON        | ~3.5ms | Larger      | Human readable         |
| MessagePack | ~1.5ms | 43% smaller | Binary                 |
| Protobuf    | Fast   | Small       | Requires .proto schema |
| RESP        | Fast   | Medium      | Text-based             |

JSON was chosen because:

- Human-readable for easier debugging
- Performance difference negligible at our message frequency
- Same serde structs work if swapping to MessagePack later
- JSON serializers escape `\n` inside strings, so no framing ambiguity

## Alternatives Considered

- **MessagePack**: fallback if JSON becomes too slow (serde makes swap trivial)
- **Protobuf**: requires `.proto` schema files and code generation, overkill
- **RESP**: designed for key-value store operations, wrong fit

## Implementation

Protocol details from Q60-Q62: newline-delimited framing, version field in every
message, lenient backward/forward compatibility.

**Status (2026-02-10):** Implemented under acd-rxy. All commands and responses
use JSON Lines. Plain text `split_whitespace()` protocol has been removed.

## Wire Format

### Commands (client -> daemon)

All commands are `IpcCommand` structs serialized as JSON Lines:

```json
{"version": 1, "cmd": "SET", "session_id": "uuid", "status": "working", "working_dir": "/path"}
{"version": 1, "cmd": "LIST"}
{"version": 1, "cmd": "GET", "session_id": "uuid"}
{"version": 1, "cmd": "RM", "session_id": "uuid"}
{"version": 1, "cmd": "SUB"}
{"version": 1, "cmd": "STATUS"}
{"version": 1, "cmd": "DUMP"}
{"version": 1, "cmd": "RESURRECT", "session_id": "uuid"}
```

### Responses (daemon -> client)

All responses are `IpcResponse` structs:

```json
{"version": 1, "ok": true, "data": ...}
{"version": 1, "ok": false, "error": "message"}
```

### SessionSnapshot (wire struct)

The `Session` struct contains `Instant` fields (not serializable).
`SessionSnapshot` is the serializable point-in-time view sent over IPC.
Conversion computes elapsed/idle seconds at the moment of serialization.

See `variable-naming.md` for naming rationale.

```text
SessionSnapshot
├── session_id: String
├── agent_type: String
├── status: String
├── working_dir: Option<String>   # None if unknown, never "unknown"
├── elapsed_seconds: u64          # since session entered current status
├── idle_seconds: u64             # since last hook activity
├── history: Vec<StatusChange>    # bounded queue, ~10 entries
└── closed: bool
```

### StatusChange (history entry)

Each entry records "became status X at time T". Consumers derive duration (diff
between consecutive `at_secs`) and previous status (prior entry's `status`). No
redundant `from` field.

```text
StatusChange
├── status: String    # the new status
└── at_secs: u64      # unix timestamp (seconds since epoch)
```

### `agent_type` serialization

The `agent_type` field in `SessionSnapshot` is produced by:

```rust
format!("{:?}", session.agent_type).to_lowercase()
```

`AgentType::ClaudeCode` serializes to `"claudecode"` (not `"claude-code"` or
`"ClaudeCode"`). This is the guaranteed wire format. Hook authors who parse the
`agent_type` field in the JSON payload must match against `"claudecode"`.

Note: the doc comment at `src/ipc.rs:172` currently says `"claude-code"` but the
actual output is `"claudecode"`. The code is correct; the comment is wrong and
will be fixed separately.

### Design Rationale

- **No `api_usage` in snapshot** — not consumed by any client yet. Add when
  implementing API usage tracking (v0 feature, not yet built).
- **No `from` in history** — redundant with previous entry's `status`. Avoids
  "from nothing" edge case on first transition.
- **`at_secs` as Unix timestamp** — self-contained, no reference time needed.
  TUI computes "5 min ago" via `now - at_secs`.

### Notifications (daemon -> SUB subscribers)

SUB clients receive `IpcNotification` JSON lines:

```json
{"version": 1, "type": "update", "session": {SessionSnapshot}}
{"version": 1, "type": "usage", "usage": {UsageData}}
{"version": 1, "type": "warn", "message": "lagged 5"}
```

[Original Q15](../archive/planning/6-open-questions.md) |
[D2](../archive/planning/discussion-decisions.md) |
[Q60-Q62](../archive/planning/6-open-questions.md)
