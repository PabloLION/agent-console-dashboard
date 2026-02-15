# Task Runner: Shell Scripts Over `just` and `cargo xtask`

Created: 20260215T190000Z Issue: acd-n7r (closed), acd-0o2

## Problem

Development tasks (test, lint, format, build, doc) need a convenient way to run.
Three options evaluated:

1. **Shell scripts** (`scripts/*.sh`)
2. **`just`** (justfile command runner)
3. **`cargo xtask`** (Rust workspace binary)

## Decision

Use shell scripts in `scripts/`.

## Comparison

### `just` (rejected)

Pros:

- Discoverability (`just --list`)
- Recipe dependencies
- Single file vs directory
- Cross-platform OS attributes (`[unix]`, `[windows]`)

Cons:

- External dependency (`brew install just`) for all contributors
- CI would need to install it (adds time, failure point)
- Our tasks are 1-2 line cargo wrappers — `just` adds a layer with marginal
  benefit
- Recipes are still shell commands (not truly cross-platform)

### `cargo xtask` (rejected)

Pros:

- Zero external dependency (Rust-native)
- Truly cross-platform (no shell needed)
- Type-safe task definitions

Cons:

- High authoring cost (10-20 lines of Rust per task vs 1-2 lines of shell)
- Extra crate in workspace
- Compilation overhead on first run
- Overkill for simple cargo wrappers

### Shell scripts (chosen)

Pros:

- Zero dependencies
- Git Bash covers Windows
- Minimal authoring cost
- Already had 3 hook scripts in `scripts/`

Cons:

- No built-in discoverability (solved with `scripts/README.md`)
- No recipe dependencies (each script is independent)
- Not truly cross-platform (but Git Bash on Windows is sufficient)

## For agents

All three options are equivalent from an agent's perspective — the agent reads
the task definitions (justfile, xtask main.rs, or scripts/README.md) and calls
the appropriate command. No difference in agent learning cost.

## Documentation

`scripts/README.md` is the single source of truth. Referenced via `@` import in
AGENTS.md (auto-loaded for agents) and linked from project README (for human
readers).
