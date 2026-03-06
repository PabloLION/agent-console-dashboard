#!/bin/sh
#
# DEPRECATED: Superseded by scripts/pre-merge-commit.sh (Git 2.24+).
#
# pre-merge-commit runs BEFORE the merge commit is created, so failures abort
# the merge cleanly. post-merge runs after — a bad merge commit already exists
# and requires a follow-up fix commit.
#
# The post-merge hook slot is now owned by the beads (bd) sync shim.
# Do not reinstall this script as .git/hooks/post-merge.
#
# This file is kept for reference and for environments that predated the
# migration. Use scripts/pre-merge-commit.sh for new setups.
#
# Original install was: ln -sf ../../scripts/post-merge.sh .git/hooks/post-merge

set -e

# Only check if Rust files were part of the merge
MERGED_RS=$(git diff --name-only HEAD~1 HEAD | grep '\.rs$' || true)

if [ -n "$MERGED_RS" ]; then
    echo "Post-merge: checking Rust formatting..."

    if ! cargo fmt --all -- --check > /dev/null 2>&1; then
        echo ""
        echo "⚠ Formatting drift detected after merge."
        echo "  Running cargo fmt --all to fix..."
        cargo fmt --all
        echo "  Fixed. Stage and commit the formatting fix before pushing."
        echo ""
    else
        echo "Post-merge: formatting OK."
    fi

    echo "Post-merge: running test suite..."
    if ! cargo test --workspace --quiet -- --test-threads=4 > /dev/null 2>&1; then
        echo ""
        echo "⚠ Tests failed after merge. Fix before pushing."
        echo "  Run 'cargo test --workspace' for details."
        echo ""
        exit 1
    else
        echo "Post-merge: all tests pass."
    fi
fi
