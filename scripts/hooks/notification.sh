#!/bin/bash
# Agent Console Dashboard - Notification Hook for Claude Code
#
# Invoked when Claude Code sends a notification (question, error, etc.).
# Sets session status to "Attention" in the dashboard.
#
# Installation:
#   1. Copy to ~/.claude/hooks/notification.sh
#   2. chmod +x ~/.claude/hooks/notification.sh
#   3. Register in ~/.claude/settings.json (see S006.04)
#
# Customization:
#   - Uncomment system notification lines below for OS-level alerts

set -euo pipefail

# Read JSON from stdin
INPUT=$(cat)

# Extract session_id from JSON (primary mechanism)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')

# Also check env var for comparison/debugging
if [ -n "${CC_SESSION_ID:-}" ] && [ "$SESSION_ID" != "$CC_SESSION_ID" ]; then
    echo "[acd-hook] Warning: JSON session_id ($SESSION_ID) differs from CC_SESSION_ID ($CC_SESSION_ID)" >&2
fi

if [ -z "$SESSION_ID" ]; then
    echo "[acd-hook] Error: No session_id found in JSON stdin" >&2
    exit 0  # Exit 0 to not block Claude Code
fi

# Update dashboard status (fail gracefully if daemon not running)
acd set "$SESSION_ID" attention || true

# Optional: System notification (uncomment one based on your OS)
#
# macOS:
# osascript -e "display notification \"Claude needs attention\" with title \"Session $SESSION_ID\""
#
# Linux (requires notify-send):
# notify-send "Agent Console" "Claude needs attention in session $SESSION_ID"
