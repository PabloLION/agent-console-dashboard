# Decision: Workspace Structure

**Decided:** 2026-01-22 **Status:** Implemented

## Context

The project needed a crate structure that separates the main binary from the
reusable usage-fetching library, allowing the library to be published
independently.

## Decision

Cargo workspace with two crates:

```text
agent-console-dashboard/
+-- Cargo.toml                    # workspace root
+-- crates/
    +-- agent-console-dashboard/  # binary crate
    |   +-- Cargo.toml
    |   +-- src/main.rs
    +-- claude-usage/             # library crate
        +-- Cargo.toml
        +-- src/lib.rs
```

The binary crate produces two binaries (`acd` and `agent-console-dashboard`)
from the same source. The `claude-usage` crate is published to crates.io as a
standalone library.

## Rationale

- `claude-usage` fills an ecosystem gap: no simple cross-platform library
  existed for fetching Claude Code usage data
- Separation isolates credential handling (security boundary)
- The library is reusable by other tools in the ecosystem
- Publishing roadmap: crates.io now, npm (via napi-rs) later

## Implementation

[Q71](../archive/planning/6-open-questions.md)
