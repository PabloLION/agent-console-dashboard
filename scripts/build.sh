#!/bin/sh
#
# Build the workspace.
#
# Usage: ./scripts/build.sh

set -e

echo "Building workspace..."
cargo build --workspace
echo "Build complete!"
