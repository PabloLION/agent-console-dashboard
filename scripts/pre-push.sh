#!/bin/sh
#
# Pre-push hook: heavier checks before pushing to remote.
# Runs cargo doc (with warnings denied) and the full test suite.
#
# Install: ln -sf ../../scripts/pre-push.sh .git/hooks/pre-push

set -e

echo "Running pre-push checks..."

echo "  Running cargo doc..."
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --quiet

echo "  Running cargo test..."
cargo test --workspace --quiet -- --test-threads=1

echo "Pre-push checks passed!"
