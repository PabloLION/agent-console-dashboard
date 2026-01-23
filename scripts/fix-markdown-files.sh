#!/bin/sh
# One-time script to format and lint all existing markdown files in docs/
# Run this ONCE to fix all existing issues, then use the pre-commit hook for future changes

set -e

echo "=== Fixing all markdown files in docs/ ==="
echo "This is a one-time operation."
echo ""

# Run prettier to auto-format
echo "Step 1: Formatting with prettier..."
if command -v prettier > /dev/null 2>&1; then
    prettier --write 'docs/**/*.md'
else
    npx prettier --write 'docs/**/*.md'
fi

echo ""

# Run markdownlint to check (not auto-fix)
echo "Step 2: Checking with markdownlint..."
if command -v markdownlint > /dev/null 2>&1; then
    markdownlint 'docs/**/*.md'
else
    npx markdownlint 'docs/**/*.md'
fi

echo ""
echo "=== Done! All markdown files in docs/ have been processed. ==="
