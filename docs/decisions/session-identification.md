# Decision: Session Identification

**Decided:** 2026-01-17 (confirmed 2026-01-31) **Status:** Implemented

## Context

Sessions need a unique identifier for tracking and a human-readable name for
display. Claude Code provides structured JSON data via stdin to hook processes,
including `session_id` and `cwd`.

## Decision

The daemon uses Claude Code's `session_id` from JSON stdin as the primary
identifier. The `display_name` is derived from `cwd` (last path component).
Every message sends the full payload (session_id, display_name, cwd, status)
with no separate registration step.

| Field          | Source                                   | Example             |
| -------------- | ---------------------------------------- | ------------------- |
| `session_id`   | From Claude Code JSON stdin              | `abc123`            |
| `display_name` | Derived from `cwd` (last path component) | `my-app`            |
| `cwd`          | From Claude Code JSON stdin              | `/Users/.../my-app` |

## Rationale

Sending the full payload every time is intentional redundancy for simplicity:

- Avoids separate register/update logic
- Handles edge cases (directory changes) automatically
- Daemon is stateless about "what fields were sent before"

The `basename "$PWD"` pattern from some early examples is stale and was replaced
with stdin JSON parsing.

## Alternatives Considered

- **UUID generation**: rejected because Claude Code already provides a
  session_id
- **Hash of cwd + PID**: rejected for same reason
- **Separate register step**: rejected because full-payload approach is simpler

## Amendments

D8 (2026-01-31) confirmed this decision and explicitly deprecated the
`basename "$PWD"` pattern in favor of JSON stdin parsing.

## Implementation

Hook scripts parse stdin JSON to extract `session_id`:

```sh
INPUT=$(cat)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id')
```

[Q16](../archive/planning/6-open-questions.md) |
[D8](../archive/planning/discussion-decisions.md)
