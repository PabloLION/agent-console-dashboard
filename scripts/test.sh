#!/bin/sh
#
# Run full test suite across the workspace.
#
# Usage: ./scripts/test.sh

set -e

echo "Running test suite..."
cargo test --workspace
echo "All tests passed!"
