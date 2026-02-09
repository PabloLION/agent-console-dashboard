# Decision: Hook Installation

**Decided:** 2026-01-22 **Status:** Implemented

## Context

Users need to add ACD hooks to Claude Code's `settings.json`. The configuration
format supports both single-object and array forms for hook events, and existing
user hooks must be preserved.

## Decision

The `acd hooks install` command uses an idempotent append algorithm to add hooks
to Claude Code's config without disrupting existing entries.

### Commands

- `acd hooks install` - adds hooks to Claude Code config
- `acd hooks uninstall` - removes only ACD hooks
- `acd hooks status` - shows if hooks are configured

### Installation Algorithm

```text
1. Read Claude Code settings.json
2. For each hook event (PreToolUse, Stop, SessionStart, SessionEnd):
   a. If event missing -> create as array with our hook
   b. If event exists as object -> convert to array, append our hook
   c. If event exists as array -> append our hook
3. Before appending, check if our hook already exists (idempotent)
   - Match by command containing "acd set --claude-hook"
   - If found -> skip (already installed)
   - If not found -> append
4. Write updated config
```

### Uninstall

Remove only entries where command contains "acd set --claude-hook". Preserve all
other hooks.

## Rationale

- Idempotent: running `install` twice produces the same result
- Handles edge case of single-object format by converting to array
- Match by command substring avoids breaking on minor flag changes
- Never modifies or removes user-configured hooks

## Implementation

[Q69](../archive/planning/6-open-questions.md)
