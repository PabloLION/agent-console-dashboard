# Story Structural Audit — 2026-01-30

**Scope:** All 55 story files in `docs/stories/`, all 13 epic files, and
`planning-artifacts/sprint-status.yaml`

**Validation script:** `docs/docs-scripts/validate-stories.sh`

## What Was Checked

- Every sprint-status entry has a corresponding story file
- Every story file has required sections (Description, Acceptance Criteria)
- Story ID in header matches filename
- Epic links in story headers resolve to existing files
- Epic story tables use hyperlinked story IDs (not plain text)
- Sprint-status statuses align with story file statuses

## Findings (Fixed)

### S003.06-client-module-internal-only.md

- Missing `## Testing Requirements` section → added
- Missing `## Out of Scope` section → added
- Epic name used "and" instead of "&" → fixed

### S008.03-claude-resume-integration.md

- Still linked to E008 but was moved to E010 → marked as "Moved to E010" with
  reference to S010.03

### Epic files (E010, E012, E013)

- Story tables had plain-text IDs without hyperlinks → added links
- E012 status was "Draft" despite being in progress → changed to "In Progress"
- E012 S012.01 status was "Draft" despite being done → changed to "Done"

### sprint-status.yaml

- E013 stories were `backlog` despite now having story files → changed to
  `ready-for-dev`

## Files Created

- `S010.03-claude-resume-in-terminal.md`
- `S012.02-health-check-command.md`
- `S012.03-diagnostic-dump-command.md`
- `S013.01-macos-launchd-plist.md`
- `S013.02-linux-systemd-unit-file.md`
- `S013.03-install-uninstall-cli.md`
- `S013.04-manual-service-setup-docs.md`

## Result

All 55 story files pass structural validation. Script exit code: 0.

## Not Yet Checked

- Deep content review of story file contents (scope, technical approach,
  acceptance criteria quality, consistency with epic goals)
- Cross-story dependency accuracy
- Estimation point consistency
