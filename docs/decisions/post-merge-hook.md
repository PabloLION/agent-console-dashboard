# Pre-Merge-Commit Git Hook

Created: 20260215T190000Z
Updated: 20260306T000000Z (migrated from post-merge to pre-merge-commit)

## Problem

Agent worktrees bypass the pre-commit hook. When the orchestrator runs
`git merge --no-ff`, the merge auto-commits without triggering pre-commit. This
means formatting drift from agents slips through to main.

Example: Rust 1.93.1 `rustfmt` wants to break long `format!()` calls across
lines. The agent's local `rustfmt` may produce different output, and the
worktree's pre-commit hook may not run or may use a different version.

## Solution

`scripts/pre-merge-commit.sh` — runs automatically before every `git merge`
commit is created (requires Git 2.24+).

### Why pre-merge-commit instead of post-merge

The original implementation used `scripts/post-merge.sh` (post-merge hook).
`post-merge` runs after the merge commit is already on the branch — a failing
check requires a follow-up fix commit to clean up main. `pre-merge-commit` runs
before the commit is created, so a failed check aborts the merge cleanly with
no commit produced.

The `post-merge` hook slot is also now occupied by the `bd` (beads) sync shim,
which handles issue tracker sync. `pre-merge-commit` is a separate slot with no
competing use.

### What it checks

1. **Formatting**: `cargo fmt --all -- --check`. If drift is detected, the
   merge is aborted. The developer must run `cargo fmt --all` and re-attempt
   the merge.
2. **Tests**: `cargo test --workspace`. If tests fail, the merge is aborted.

### When it fires

Only when Rust files (`.rs`) differ between the current branch tip and the
incoming branch tip (`git diff HEAD MERGE_HEAD -- '*.rs'`). Skips entirely for
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
ln -sf ../../scripts/pre-merge-commit.sh .git/hooks/pre-merge-commit
```
