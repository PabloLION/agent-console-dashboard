#!/bin/sh
#
# Install the agent-console-dashboard binary (acd).
#
# Pre-flight checks:
# - Verify cargo is installed
# - Verify rust toolchain is available
#
# Note: This script does NOT run `acd install` — user must run that manually
# after installation to set up git hooks and configuration.
#
# Usage: ./scripts/install.sh

set -e

echo "Running pre-flight checks..."

# Check if cargo is installed
if ! command -v cargo >/dev/null 2>&1; then
    echo "Error: cargo is not installed or not in PATH"
    echo "Install Rust from https://rustup.rs/"
    exit 1
fi

# Check if rustc is available
if ! command -v rustc >/dev/null 2>&1; then
    echo "Error: rustc is not installed or not in PATH"
    echo "Install Rust from https://rustup.rs/"
    exit 1
fi

echo "Pre-flight checks passed!"
echo ""
echo "Installing agent-console-dashboard..."
cargo install --path crates/agent-console-dashboard

echo ""
echo "Installation complete!"
echo ""
echo "Next steps:"
echo "  1. Run 'acd install' to set up git hooks and configuration"
echo "  2. Run 'acd --help' to see available commands"
