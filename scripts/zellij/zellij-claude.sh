#!/bin/bash
# Zellij layout launcher for Agent Console Dashboard

set -euo pipefail

if [ -n "${ZELLIJ:-}" ]; then
    echo "Already inside Zellij session."
    echo "Use 'zellij action new-pane' to add dashboard to current session."
    exit 1
fi

if ! command -v zellij &> /dev/null; then
    echo "Error: Zellij not installed. See https://zellij.dev/documentation/installation"
    exit 1
fi

# Version check (warn if < 0.39.0)
ZELLIJ_VERSION=$(zellij --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "unknown")
if [ "$ZELLIJ_VERSION" != "unknown" ]; then
    MAJOR=$(echo "$ZELLIJ_VERSION" | cut -d. -f1)
    MINOR=$(echo "$ZELLIJ_VERSION" | cut -d. -f2)
    if [ "$MAJOR" -eq 0 ] && [ "$MINOR" -lt 39 ]; then
        echo "Warning: Zellij $ZELLIJ_VERSION detected. Version 0.39.0+ recommended."
    fi
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LAYOUT="${1:-default}"

case "$LAYOUT" in
    "minimal"|"1")
        LAYOUT_FILE="$SCRIPT_DIR/layouts/claude-minimal.kdl"
        ;;
    "default"|"2")
        LAYOUT_FILE="$SCRIPT_DIR/layouts/claude-default.kdl"
        ;;
    "detailed"|"3")
        LAYOUT_FILE="$SCRIPT_DIR/layouts/claude-detailed.kdl"
        ;;
    *)
        echo "Usage: $0 [minimal|default|detailed]"
        echo ""
        echo "Layouts:"
        echo "  minimal   1-line dashboard (session names + status)"
        echo "  default   2-line dashboard (status + working directory)"
        echo "  detailed  Multi-line dashboard (full session details)"
        exit 1
        ;;
esac

if [ ! -f "$LAYOUT_FILE" ]; then
    echo "Error: Layout file not found: $LAYOUT_FILE"
    exit 1
fi

zellij --layout "$LAYOUT_FILE"
