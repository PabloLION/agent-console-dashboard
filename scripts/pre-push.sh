#!/bin/sh
#
# Pre-push hook: checks before pushing to remote.
# Chains to global hooks first, then runs cargo doc.
#
# Install: ln -sf ../../scripts/pre-push.sh .git/hooks/pre-push

set -e

# ---------------------------------------------------------------------------
# Chain to global hooks (if configured)
# ---------------------------------------------------------------------------

GLOBAL_HOOKS_DIR=$(git config --global core.hooksPath 2>/dev/null || true)
if [ -n "$GLOBAL_HOOKS_DIR" ]; then
    HOOK_NAME=$(basename "$0")
    GLOBAL_HOOK="$GLOBAL_HOOKS_DIR/$HOOK_NAME"
    if [ -x "$GLOBAL_HOOK" ]; then
        "$GLOBAL_HOOK" "$@"
    fi
fi

echo "Running pre-push checks..."

echo "  Running cargo doc..."
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --quiet

echo "Pre-push checks passed!"
