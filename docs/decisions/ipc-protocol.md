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

[Original Q15](../archive/planning/6-open-questions.md) |
[D2](../archive/planning/discussion-decisions.md) |
[Q60-Q62](../archive/planning/6-open-questions.md)
