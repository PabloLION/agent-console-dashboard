# Development Scripts

Shell scripts for common development tasks. All scripts use `#!/bin/sh` and
`set -e`.

## Development Commands

| Script | Description | Usage |
| --- | --- | --- |
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
| `post-merge.sh` | post-merge | Check formatting drift + run tests after merge |

### Installing git hooks

```sh
ln -sf ../../scripts/pre-commit.sh .git/hooks/pre-commit
ln -sf ../../scripts/pre-push.sh .git/hooks/pre-push
ln -sf ../../scripts/post-merge.sh .git/hooks/post-merge
```
