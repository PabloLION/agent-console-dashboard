#!/bin/sh
#
# Auto-fix formatting across the workspace.
#
# Usage: ./scripts/fmt.sh

set -e

echo "Running cargo fmt..."
cargo fmt --all
echo "Formatting complete!"
