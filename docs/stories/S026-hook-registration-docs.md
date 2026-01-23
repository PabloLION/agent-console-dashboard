# Story: Document Hook Registration in settings.json

**Story ID:** S026
**Epic:** [E006 - Claude Code Integration](../epic/E006-claude-code-integration.md)
**Status:** Draft
**Priority:** P2
**Estimated Points:** 1

## Description

As a user,
I want clear documentation on how to register hooks in Claude Code settings,
So that I can easily set up the dashboard integration with my Claude Code installation.

## Context

The hook scripts (S023-S025) need to be registered in Claude Code's settings file before they will be invoked. This story creates comprehensive documentation that guides users through the registration process, including file locations, JSON format, troubleshooting, and verification steps.

Good documentation is essential for adoption - users shouldn't have to guess how to configure hooks or debug registration issues.

## Implementation Details

### Technical Approach

1. Create `docs/integration/claude-code-hooks.md` comprehensive guide
2. Document the settings.json location and format
3. Provide step-by-step installation instructions
4. Include troubleshooting section for common issues
5. Add verification commands to confirm hooks are working
6. Update README.md with quick-start hook setup

### Files to Modify

- `docs/integration/claude-code-hooks.md` - Create comprehensive hook documentation
- `README.md` - Add quick-start section for hook setup
- `scripts/hooks/README.md` - Document individual hook scripts

### Dependencies

- [S023 - Stop Hook Script](./S023-stop-hook-script.md) - Hook scripts must exist
- [S024 - User Prompt Submit Hook](./S024-user-prompt-submit-hook.md) - Hook scripts must exist
- [S025 - Notification Hook Script](./S025-notification-hook-script.md) - Hook scripts must exist

## Acceptance Criteria

- [ ] Given a new user reads the documentation, when they follow the steps, then hooks are correctly registered
- [ ] Given the settings.json format, when documented, then all required fields and arrays are explained
- [ ] Given a user has registration issues, when they check troubleshooting section, then common problems are addressed
- [ ] Given hooks are registered, when user runs verification commands, then they can confirm hooks work
- [ ] Given the documentation exists, when linked from README.md, then users can easily find it
- [ ] Given Claude Code's settings.json location varies by OS, when documented, then all platforms are covered

## Testing Requirements

- [ ] Manual test: Follow documentation steps on a clean Claude Code installation
- [ ] Manual test: Verify all code examples are syntactically correct
- [ ] Manual test: Test verification commands work as documented
- [ ] Review: Have someone unfamiliar with the project follow the guide

## Out of Scope

- Automatic registration (users edit settings.json manually)
- GUI configuration tool
- Hook management commands in agent-console CLI
- Windows-specific documentation

## Notes

### Documentation Structure

```markdown
# Claude Code Hooks Integration

## Overview
Brief explanation of what hooks do and why they're useful.

## Prerequisites
- Agent Console Dashboard installed
- Claude Code installed
- Bash shell available

## Quick Start
Minimal steps to get hooks working.

## Detailed Setup

### Step 1: Copy Hook Scripts
Where to copy scripts and how to make them executable.

### Step 2: Configure settings.json
Full settings.json example and explanation.

### Step 3: Verify Installation
How to test that hooks are working.

## Settings.json Reference
Complete documentation of hook configuration options.

## Troubleshooting
Common issues and solutions.

## Advanced Configuration
Custom hooks, multiple instances, etc.
```

### Settings.json Location

| Platform | Path |
|----------|------|
| macOS | `~/.claude/settings.json` |
| Linux | `~/.claude/settings.json` |
| Windows | `%USERPROFILE%\.claude\settings.json` |

### Example settings.json

```json
{
  "hooks": {
    "Stop": ["~/.claude/hooks/stop.sh"],
    "UserPromptSubmit": ["~/.claude/hooks/user-prompt-submit.sh"],
    "Notification": ["~/.claude/hooks/notification.sh"]
  }
}
```

### Hook Registration Format

The `hooks` object maps hook names to arrays of script paths:

```json
{
  "hooks": {
    "<HookName>": ["<path/to/script1>", "<path/to/script2>"]
  }
}
```

- Hook names are case-sensitive: `Stop`, `UserPromptSubmit`, `Notification`
- Each hook can have multiple scripts (executed in order)
- Scripts can be absolute paths or `~` for home directory
- Scripts must be executable (`chmod +x`)

### Troubleshooting Section Topics

1. **Hooks not firing**
   - Check settings.json syntax (valid JSON)
   - Verify hook name spelling (case-sensitive)
   - Ensure scripts are executable

2. **Dashboard not updating**
   - Verify daemon is running: `agent-console list`
   - Check agent-console is in PATH
   - Test script manually: `./stop.sh`

3. **Permission denied**
   - Run `chmod +x ~/.claude/hooks/*.sh`
   - Check file ownership

4. **Wrong project name**
   - Hooks use `basename $PWD`
   - Ensure Claude Code runs from project root

5. **Multiple hooks issue**
   - Each hook type is an array
   - Check for duplicate entries

### Verification Commands

```bash
# Test that agent-console is working
agent-console list

# Manually trigger a status update
agent-console set test-project attention

# Check dashboard shows the update
agent-console tui

# Clean up test
agent-console rm test-project
```

### Integration with Existing settings.json

If the user already has a settings.json with other configurations:

```json
{
  "existingSetting": "value",
  "hooks": {
    "Stop": ["~/.claude/hooks/stop.sh"]
  }
}
```

The documentation should explain how to merge hook configuration with existing settings without overwriting other configurations.
