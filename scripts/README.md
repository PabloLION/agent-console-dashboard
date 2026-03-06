# Development Scripts

Shell scripts for common development tasks. All scripts use `#!/bin/sh` and
`set -e`.

## Development Commands

| Script | Description | Usage |
| --- | --- | --- |
| `install.sh` | Install acd binary with pre-flight checks | `./scripts/install.sh` |
| `test.sh` | Run full test suite | `./scripts/test.sh` |
| `lint.sh` | Check formatting + clippy (read-only) | `./scripts/lint.sh` |
| `fmt.sh` | Auto-fix formatting | `./scripts/fmt.sh` |
| `build.sh` | Build workspace | `./scripts/build.sh` |
| `doc.sh` | Build documentation (no deps) | `./scripts/doc.sh` |

## Git Hooks

| Script | Hook | Description |
| --- | --- | --- |
| `pre-commit.sh` | pre-commit | Format, lint, and test staged Rust/Markdown files |
| `pre-push.sh` | pre-push | Build documentation before push |
| `pre-merge-commit.sh` | pre-merge-commit | Check formatting + run tests before merge commit (Git 2.24+) |
| `post-merge.sh` | — | **Deprecated.** Superseded by `pre-merge-commit.sh`. |

The `post-merge` hook slot is managed by `bd` (beads) for issue tracker sync.
The ACD formatting and test checks run in `pre-merge-commit` instead, which
aborts the merge before any commit is created if checks fail.

### Installing git hooks

```sh
ln -sf ../../scripts/pre-commit.sh .git/hooks/pre-commit
ln -sf ../../scripts/pre-push.sh .git/hooks/pre-push
ln -sf ../../scripts/pre-merge-commit.sh .git/hooks/pre-merge-commit
```
