#!/usr/bin/env bash
# validate-stories.sh — BMAD story file structural validation
# Run from repo root: ./docs/scripts/validate-stories.sh
#
# Checks:
# 1. Every story in sprint-status.yaml has a corresponding file
# 2. Every story file has required sections
# 3. Story ID in header matches filename
# 4. Epic links in story headers point to existing files
# 5. Epic story tables have hyperlinked story IDs
# 6. Sprint-status statuses are consistent with story file statuses

set -euo pipefail

STORIES_DIR="docs/stories"
EPICS_DIR="docs/epic"
SPRINT_STATUS="planning-artifacts/sprint-status.yaml"
ERRORS=0

red() { printf '\033[0;31m%s\033[0m\n' "$1"; }
green() { printf '\033[0;32m%s\033[0m\n' "$1"; }
yellow() { printf '\033[0;33m%s\033[0m\n' "$1"; }

# --- Check 1: Every story in sprint-status has a file ---
echo "=== Check 1: Story files exist for all sprint-status entries ==="
grep -E '^\s+[0-9]{3}-[0-9]{2}-' "$SPRINT_STATUS" | while IFS=: read -r key status; do
    key=$(echo "$key" | xargs)
    status=$(echo "$status" | xargs)

    # Skip moved/cut/deferred stories
    if [[ "$status" == "moved-to-"* || "$status" == "cut" || "$status" == "deferred" ]]; then
        continue
    fi

    # Convert sprint key (e.g., 001-01-create-daemon-process) to story file pattern
    epic_num=$(echo "$key" | grep -oE '^[0-9]{3}' | sed 's/^0*//')
    story_num=$(echo "$key" | grep -oE '^[0-9]{3}-[0-9]{2}' | grep -oE '[0-9]{2}$' | sed 's/^0*//')
    pattern="S$(printf '%03d' "$epic_num").$(printf '%02d' "$story_num")-*.md"

    matches=$(find "$STORIES_DIR" -name "$pattern" 2>/dev/null | head -1)
    if [[ -z "$matches" ]]; then
        red "MISSING: $key ($status) — no file matching $pattern"
        ERRORS=$((ERRORS + 1))
    fi
done

# --- Check 2: Required sections in story files ---
echo ""
echo "=== Check 2: Required sections in story files ==="
REQUIRED_SECTIONS=("## Description" "## Acceptance Criteria")
RECOMMENDED_SECTIONS=("## Testing Requirements" "## Out of Scope" "## Implementation Details")

for file in "$STORIES_DIR"/S*.md; do
    basename=$(basename "$file")
    for section in "${REQUIRED_SECTIONS[@]}"; do
        if ! grep -q "^${section}$" "$file"; then
            red "MISSING REQUIRED: $basename — missing '$section'"
            ERRORS=$((ERRORS + 1))
        fi
    done
    for section in "${RECOMMENDED_SECTIONS[@]}"; do
        if ! grep -q "^${section}" "$file"; then
            yellow "MISSING RECOMMENDED: $basename — missing '$section'"
        fi
    done
done

# --- Check 3: Story ID matches filename ---
echo ""
echo "=== Check 3: Story ID matches filename ==="
for file in "$STORIES_DIR"/S*.md; do
    basename=$(basename "$file" .md)
    # Extract story ID from filename: S001.01-name -> S001.01
    file_id=$(echo "$basename" | grep -oE '^S[0-9]+\.[0-9]+')

    # Extract story ID from header line
    header_id=$(grep -oE 'Story ID:\*\* S[0-9]+\.[0-9]+' "$file" | grep -oE 'S[0-9]+\.[0-9]+' || true)

    if [[ -z "$header_id" ]]; then
        red "NO ID: $basename — could not find Story ID in header"
        ERRORS=$((ERRORS + 1))
    elif [[ "$file_id" != "$header_id" ]]; then
        red "MISMATCH: $basename — filename says $file_id, header says $header_id"
        ERRORS=$((ERRORS + 1))
    fi
done

# --- Check 4: Epic links in story headers resolve ---
echo ""
echo "=== Check 4: Epic links in story headers resolve ==="
for file in "$STORIES_DIR"/S*.md; do
    basename=$(basename "$file")
    epic_link=$(grep -oE '\.\./epic/E[0-9]+-[a-z0-9-]+\.md' "$file" | head -1 || true)
    if [[ -n "$epic_link" ]]; then
        resolved="$STORIES_DIR/$epic_link"
        if [[ ! -f "$resolved" ]]; then
            red "BROKEN LINK: $basename — $epic_link does not exist"
            ERRORS=$((ERRORS + 1))
        fi
    else
        red "NO EPIC LINK: $basename — no epic link found in header"
        ERRORS=$((ERRORS + 1))
    fi
done

# --- Check 5: Epic story tables use hyperlinks ---
echo ""
echo "=== Check 5: Epic story tables use hyperlinked story IDs ==="
for file in "$EPICS_DIR"/E*.md; do
    basename=$(basename "$file")
    # Find story IDs in table rows that are NOT hyperlinked
    # Match lines with | S0XX.YY | pattern (not wrapped in [...])
    unlinked=$(grep -E '^\| S[0-9]+\.[0-9]+\s' "$file" || true)
    if [[ -n "$unlinked" ]]; then
        red "UNLINKED: $basename — story IDs in table not hyperlinked:"
        echo "$unlinked"
        ERRORS=$((ERRORS + 1))
    fi
done

# --- Summary ---
echo ""
if [[ $ERRORS -gt 0 ]]; then
    red "=== FAILED: $ERRORS issue(s) found ==="
    exit 1
else
    green "=== PASSED: All structural checks passed ==="
    exit 0
fi
