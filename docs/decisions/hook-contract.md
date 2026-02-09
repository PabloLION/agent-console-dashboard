# Decision: Hook Contract

**Decided:** 2026-01-22 **Status:** Implemented

## Context

Claude Code hooks are blocking (Claude waits for them to complete). The hook
command's exit code and timeout behavior directly affect the user's Claude Code
experience. A misbehaving hook could block or confuse Claude.

## Decision

### Exit Codes

| Exit Code | Behavior           | stdout                         | stderr                        |
| --------- | ------------------ | ------------------------------ | ----------------------------- |
| 0         | Success            | Shown in verbose mode (Ctrl+O) | Ignored                       |
| 2         | Blocking error     | Ignored                        | Error message shown to Claude |
| Other     | Non-blocking error | Ignored                        | Shown with warning prefix     |

ACD hooks never exit 2 (that would block Claude from proceeding).

### Timeout

5 seconds recommended. Configurable per hook in Claude Code's hook config (not
controlled by ACD). Claude Code's own default is 60 seconds.

| Scenario           | Behavior                           |
| ------------------ | ---------------------------------- |
| Daemon unreachable | `acd set` exits 1 after 5s timeout |
| Socket write hangs | `acd set` exits 1 after 5s timeout |
| Daemon responds    | `acd set` exits 0 immediately      |

### Question Detection (Q74)

The `tool_name` field in PreToolUse stdin JSON identifies AskUserQuestion:

| Hook Event   | tool_name       | Status    |
| ------------ | --------------- | --------- |
| PreToolUse   | AskUserQuestion | question  |
| PreToolUse   | (any other)     | working   |
| Stop         | -               | attention |
| SessionEnd   | -               | closed    |
| SessionStart | -               | working   |

### Error Handling

On failure, `acd set` broadcasts the error to all connected dashboards (not to
Claude, which would waste context space). Dashboard shows "Hook error: daemon
unreachable" or similar.

## Rationale

- Exit 1 (non-blocking) ensures Claude always continues regardless of ACD health
- 5s timeout is fast enough to avoid noticeable delay in Claude's workflow
- Broadcasting errors to dashboards keeps the user informed without polluting
  Claude's context

## Implementation

Claude Code v2.0.76 or later required for reliable AskUserQuestion hook
detection.

[Q50](../archive/planning/6-open-questions.md) |
[Q29](../archive/planning/6-open-questions.md) |
[Q74](../archive/planning/6-open-questions.md) |
[Q7 in 7-decisions](../archive/planning/7-decisions.md)
