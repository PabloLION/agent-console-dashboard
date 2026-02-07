#!/bin/sh
#
# Pre-commit hook: checks staged files before committing.
# Runs markdown checks on .md files and Rust checks on .rs files.
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

    echo "  Running cargo fmt --check..."
    cargo fmt --all -- --check

    echo "  Running cargo clippy..."
    cargo clippy --workspace -- -D warnings

    echo "Rust checks passed!"
fi
