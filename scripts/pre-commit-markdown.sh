#!/bin/sh
#
# This hook was set by cargo-husky v1.5.0: https://github.com/rhysd/cargo-husky#readme
# Pre-commit hook for markdown files
# Only checks STAGED markdown files (not all files in docs/)

# Get staged markdown files
STAGED_MD=$(git diff --cached --name-only --diff-filter=ACM | grep '\.md$' || true)

# Exit early if no markdown files staged
if [ -z "$STAGED_MD" ]; then
    exit 0
fi

echo "Checking staged markdown files..."

# Run prettier on staged files only
if command -v prettier > /dev/null 2>&1; then
    echo "$STAGED_MD" | xargs prettier --check
else
    echo "$STAGED_MD" | xargs npx prettier --check
fi

# Run markdownlint on staged files only
if command -v markdownlint > /dev/null 2>&1; then
    echo "$STAGED_MD" | xargs markdownlint
else
    echo "$STAGED_MD" | xargs npx markdownlint
fi

echo "Markdown checks passed!"
