# Session Update: Single Command with Optional Flags

Created: 20260215T120000Z Issue: acd-2jp (CLI tree redesign)

## Problem

The CLI needs a command to modify session fields (status, priority,
working-dir). Two design axes:

1. **Naming**: `set` vs `update`
2. **Atomicity**: one command for all fields, or separate commands per field

## Decision

Single `update` command with optional flags:

```sh
acd session update <id> [--status=<status>] [--priority=<n>] [--working-dir=<path>]
```

All flags are optional. At least one must be provided. The daemon applies all
changes in a single operation.

## Naming: `update` over `set`

Both verbs mean "change fields on an existing session" in our context (CLI
errors on nonexistent sessions — lazy-create is hooks-only). But `update` is the
conventional CRUD verb for partial modification. `set` implies assigning a value
to something that may be empty, which doesn't match our use case since sessions
already exist when the CLI operates on them.

## Atomicity: Single Command over Multiple

Considered two approaches:

### A. Separate commands per field

```sh
acd session set-status <id> working
acd session set-priority <id> 5
```

Each command does exactly one thing. Simpler to understand individually.

Rejected because:

- **Non-atomic**: two IPC round-trips create a window where status changed but
  priority didn't. TUI observers see inconsistent state.
- **More typing**: common case is changing status + priority together.
- **Unconventional**: no major CLI tool uses this pattern (kubectl, gh, docker
  all use `update` with flags).

### B. Single update command (chosen)

```sh
acd session update <id> --status=working --priority=5
```

One IPC call, one daemon operation, one response. Atomic from the observer's
perspective. Follows established CLI conventions.

## Edge Cases

- **No flags provided**: warning message ("nothing to update"), not an error.
- **Nonexistent session**: error. CLI does not lazy-create (hooks-only).
- **Invalid status/priority**: validation error before IPC call.

## Context

This command lives under the `session` subgroup in the CLI tree redesign
(acd-2jp):

```text
acd session
├── update <id> [--status] [--priority] [--working-dir]
├── reopen <id>
└── list
```

The existing `acd set <id> <status>` command is replaced by
`acd session update <id> --status=<status>`.
