# Pending User Scope Changes

Proposed changes to user scope settings (`~/.claude/rules/*`). These should be
reviewed and applied by the dedicated agent, not by project agents directly.

## Changes

### User Scope Safety Rule

**Target**: New file or addition to existing high-priority rules file
**Priority**: High — should rank above most other rules

Always pause and show the proposed change before editing user scope files
(`~/.claude/CLAUDE.md`, `~/.claude/rules/*`). These files affect all projects —
changes require explicit user approval.

Project agents should never directly edit user scope files. Instead, they should
document proposed changes in a project-level file (like this one) for the
dedicated agent to process.

### Compact ISO 8601 Date Format Definition

**Target**: User scope definitions or terminology

Compact timestamp format: `YYYYMMDD-HHmmss` (15 characters). ISO 8601 basic
format with dash separator between date and time. Example: `20260212-143052`.
Use for backup file suffixes and other machine-generated timestamps.
