#!/bin/sh
#
# Build documentation for the workspace (excludes dependencies).
#
# Usage: ./scripts/doc.sh

set -e

echo "Building documentation..."
cargo doc --workspace --no-deps
echo "Documentation built!"
