# Subagent Permissions for Background Dispatch

Research date: 2026-02-12

## Problem

Background agents (Task tool with `run_in_background: true`) get all tool
permissions auto-denied because there's no interactive approval available.

## Findings

- Background agents **pre-approve permissions upfront** before launching
- Once running, they auto-deny anything not pre-approved
- The `permissions.allow` list in `settings.local.json` controls what's
  pre-approved
- **Settings changes require a session restart** to take effect — modifying
  `settings.local.json` mid-session has no effect on already-launched or future
  agents in the same session

## Permission Format

```text
Read(//tmp/acd-wt-**)     — absolute path (note double //)
Edit(.claude/**)          — relative to project root
Bash(cargo test:*)        — Bash command with wildcard description
```

Pattern types:

- `//path` — absolute filesystem path
- `~/path` — home directory relative
- `/path` — relative to settings file location
- `./path` or `path` — relative to cwd

## Applied Configuration

Added to `.claude/settings.local.json`:

- `Read/Edit/Write(//tmp/acd-wt-**)` — worktree file access
- `Read/Edit/Write(.claude/**)` — agent memory access
- Broader Bash: `git status/diff/branch/stash/rm/worktree`, `mkdir`, `ls`, `bd`

## Lesson

Always restart the session after modifying permission settings. Test with a
simple agent before dispatching a full batch.
