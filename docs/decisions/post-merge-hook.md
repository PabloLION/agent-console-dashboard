# Post-Merge Git Hook

Created: 20260215T190000Z

## Problem

Agent worktrees bypass the pre-commit hook. When the orchestrator runs
`git merge --no-ff`, the merge auto-commits without triggering pre-commit. This
means formatting drift from agents slips through to main.

Example: Rust 1.93.1 `rustfmt` wants to break long `format!()` calls across
lines. The agent's local `rustfmt` may produce different output, and the
worktree's pre-commit hook may not run or may use a different version.

## Solution

`scripts/post-merge.sh` — runs automatically after every `git merge`.

### What it checks

1. **Formatting**: `cargo fmt --all -- --check`. If drift detected, auto-fixes
   with `cargo fmt --all` and tells the user to commit the fix.
2. **Tests**: `cargo test --workspace`. If tests fail, exits non-zero and tells
   the user to fix before pushing.

### When it fires

Only when Rust files (`.rs`) were part of the merge. Skips entirely for
documentation-only or config-only merges.

## Trade-off

The hook adds ~10-15s to every merge that includes Rust files (compilation +
test run). This is acceptable because:

- Merges happen infrequently (once per agent completion)
- Catching formatting drift and test failures at merge time prevents CI failures
- The alternative (manual `cargo fmt --check` in orchestrator rules) consumes
  agent context tokens and is easy to forget

## Install

```sh
ln -sf ../../scripts/post-merge.sh .git/hooks/post-merge
```
