# Pre-Commit Hooks

This document records the design decisions for pre-commit hooks in ACD.

## Decision

- **`cargo fmt`**: Auto-fix and re-stage in pre-commit hook (tracked as acd-ir7)
- **`cargo clippy`**: Report-only gate (no auto-fix)

## Rationale

### Auto-fix for fmt

Formatting is mechanical and deterministic. Reporting without fixing adds an
unnecessary manual step.

- `cargo fmt` output is always predictable and correct
- No human judgment required
- Auto-fixing saves time and reduces friction

### Gate-only for clippy

`cargo clippy --fix` is unreliable — can introduce compilation errors or make
wrong choices.

- Clippy suggestions require human judgment
- Auto-applying fixes can break code
- Report-only is safer and allows developer review

## Partial Staging

**Open question**: How to handle partial staging (staged vs unstaged changes in
same file)?

**Current stance**: ACD workflow always commits whole files, so `git add -u`
after fmt is safe.

**If partial staging is ever needed**: Revisit this decision. May need to track
which hunks were originally staged and only re-stage formatted versions of those
hunks.

## Pre-Stage Hook

**No pre-stage hook**: Git has no native pre-stage hook.

**Workarounds exist** (shell alias, git wrapper) but are fragile. The pre-commit
approach is more reliable.

## Implementation Status

Tracked as acd-ir7.
