#!/bin/sh
#
# Pre-commit hook: checks staged files before committing.
#
# Install: ln -sf ../../scripts/pre-commit.sh .git/hooks/pre-commit

set -e

# ---------------------------------------------------------------------------
# Markdown checks
# ---------------------------------------------------------------------------

STAGED_MD=$(git diff --cached --name-only --diff-filter=ACM | grep '\.md$' || true)

if [ -n "$STAGED_MD" ]; then
    echo "Checking staged markdown files..."

    if command -v prettier > /dev/null 2>&1; then
        echo "$STAGED_MD" | xargs prettier --check
    else
        echo "$STAGED_MD" | xargs npx prettier --check
    fi

    if command -v markdownlint > /dev/null 2>&1; then
        echo "$STAGED_MD" | xargs markdownlint
    else
        echo "$STAGED_MD" | xargs npx markdownlint
    fi

    echo "Markdown checks passed!"
fi

# ---------------------------------------------------------------------------
# Rust checks
# ---------------------------------------------------------------------------

STAGED_RS=$(git diff --cached --name-only --diff-filter=ACM | grep '\.rs$' || true)

if [ -n "$STAGED_RS" ]; then
    echo "Checking Rust files..."

    echo "  Running cargo fmt (auto-fix)..."
    cargo fmt --all
    git add -u

    echo "  Running cargo clippy..."
    cargo clippy --workspace -- -D warnings

    echo "  Running cargo test..."
    # Capture test output; filter noise on success, show full output on failure.
    # TODO: migrate to justfile recipe (acd-n7r)
    TEST_OUTPUT=$(mktemp)
    if cargo test --workspace --quiet -- --test-threads=4 > "$TEST_OUTPUT" 2>&1; then
        # Success: remove lines consisting entirely of dots/ignored markers (i),
        # optionally followed by a progress counter (e.g., "... 174/625").
        # Keep everything else (warnings, errors, headers, summaries).
        grep -Ev '^[.i]+( [0-9]+/[0-9]+)?$' "$TEST_OUTPUT" || true
        rm "$TEST_OUTPUT"
    else
        # Failure: show full output so developer can see what failed
        cat "$TEST_OUTPUT"
        rm "$TEST_OUTPUT"
        exit 1
    fi

    echo "Rust checks passed!"
fi
