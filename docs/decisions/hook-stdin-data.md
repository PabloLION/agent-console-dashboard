# Decision: Hook Stdin Data Flow

**Decided:** 2026-01-22 **Status:** Implemented

## Context

Claude Code passes structured JSON data to hook processes via stdin. The hook
command needs to parse this data, extract relevant fields, and forward a status
update to the daemon. A design was needed for how `acd set` handles this input
and how it supports future multi-agent scenarios.

## Decision

`acd set` parses stdin JSON and uses a `--source` flag to select the parser.

### Data Flow

```text
Claude Code hook fires
    |
stdin: { "session_id": "abc", "cwd": "/path", "hook_event_name": "PreToolUse", ... }
    |
acd set --source claude-code
    |
Parse stdin JSON (using claude-code parser), extract:
  - session_id
  - cwd
  - hook_event_name -> map to status
  - tool_name -> check for AskUserQuestion
    |
Send to daemon: { session_id, cwd, status }
```

### Process Lifetime

`acd set` is short-lived (~1ms). It parses stdin, sends one message to the
daemon, and exits. No RAM accumulation across invocations.

### Multi-Agent Support

Each agent source has its own parser producing the same output fields:

| Agent       | Command                        |
| ----------- | ------------------------------ |
| Claude Code | `acd set --source claude-code` |
| Gemini CLI  | `acd set --source gemini`      |
| Codex CLI   | `acd set --source codex`       |

### Error Handling

If stdin is empty or invalid JSON, `acd set` logs a warning and exits 1
(non-blocking error per the hook contract). The daemon receives no update, but
Claude continues unaffected.

## Rationale

- Stdin JSON already contains `hook_event_name`, so no separate event flag
  needed
- The `--source` flag enables future multi-agent support without protocol
  changes
- Short process lifetime means negligible resource overhead per hook invocation

## Implementation

[Q73](../archive/planning/6-open-questions.md) |
[Hook contract](hook-contract.md)
