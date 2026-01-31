#!/bin/bash
# Agent Console Dashboard - UserPromptSubmit Hook for Claude Code
#
# Invoked when a user submits a prompt to Claude Code.
# Sets session status to "Working" in the dashboard.
#
# Installation:
#   1. Copy to ~/.claude/hooks/user-prompt-submit.sh
#   2. chmod +x ~/.claude/hooks/user-prompt-submit.sh
#   3. Register in ~/.claude/settings.json (see S006.04)

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
acd set "$SESSION_ID" working || true
