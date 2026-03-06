#!/bin/sh
#
# Pre-merge-commit hook: verify formatting and run tests before the merge
# commit is created.
#
# Runs before git creates the merge commit, so a failed check aborts the
# merge cleanly — nothing lands on the branch. This is safer than
# post-merge, where a bad merge commit exists and must be reverted.
#
# Requires Git 2.24+ (first version to support pre-merge-commit).
#
# Install: ln -sf ../../scripts/pre-merge-commit.sh .git/hooks/pre-merge-commit

set -e

# Only check if Rust files are part of the incoming changes.
# MERGE_HEAD is the tip of the branch being merged in; HEAD is the current
# branch tip. The diff between them shows what this merge introduces.
MERGED_RS=$(git diff --name-only HEAD MERGE_HEAD -- '*.rs' 2>/dev/null || true)

if [ -n "$MERGED_RS" ]; then
    echo "Pre-merge-commit: checking Rust formatting..."

    if ! cargo fmt --all -- --check > /dev/null 2>&1; then
        echo ""
        echo "ERROR: Formatting drift detected in merged result."
        echo "  Run 'cargo fmt --all' to fix, then retry the merge."
        echo ""
        exit 1
    else
        echo "Pre-merge-commit: formatting OK."
    fi

    echo "Pre-merge-commit: running test suite..."
    if ! cargo test --workspace --quiet -- --test-threads=4 > /dev/null 2>&1; then
        echo ""
        echo "ERROR: Tests failed in merged result. Fix before merging."
        echo "  Run 'cargo test --workspace' for details."
        echo ""
        exit 1
    else
        echo "Pre-merge-commit: all tests pass."
    fi
fi
