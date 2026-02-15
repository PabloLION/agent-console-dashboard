#!/bin/sh
#
# Check formatting and run clippy across the workspace.
# Does not modify files — use fmt.sh to auto-fix formatting.
#
# Usage: ./scripts/lint.sh

set -e

echo "Checking formatting..."
cargo fmt --all -- --check

echo "Running clippy..."
cargo clippy --workspace -- -D warnings

echo "Lint checks passed!"
