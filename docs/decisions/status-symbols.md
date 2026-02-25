# Status Symbols

**Decided:** 2026-02-24 **Status:** Implemented

## Decision

Status symbols are single ASCII characters:

```csv
Symbol,Status,Meaning
*,Working,Agent is actively processing
!,Attention,Agent needs user input
?,Question,Agent asked a question
x,Closed,Session has ended
.,Inactive,Session is idle (derived, not a status variant)
```

Symbols appear in two-line chip display and in the status column of the large
layout.

## Rationale

ASCII characters work in every terminal environment: SSH sessions, older
terminal emulators, and systems with incomplete Unicode font support. Unicode
symbols such as `●`, `⚠`, `✓` are visually appealing but fail or render as
replacement characters in constrained environments.

ACD runs as a background monitor. It must display correctly on the machines
where Claude Code runs, which are not always the user's primary workstation with
full font support.

## Alternatives Considered

- **Unicode symbols** (`●`, `⚠`, `✓`, `✗`, `·`): More visually distinct.
  Rejected because they are not reliably renderable across SSH sessions, older
  terminal emulators, and systems without complete Unicode font coverage.
- **Colored blocks without symbols**: Use background color only, no character.
  Rejected because color-blind users would lose all differentiation.
