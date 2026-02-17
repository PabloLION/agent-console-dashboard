# Cancel Event Investigation

Research for acd-4cqb. Conducted: 2026-02-17.

## Summary

**Likely Yes, but empirical verification needed.** The `Stop` hook event is
already registered in ACD (build.rs) and maps to "attention" status. It should
fire on ESC/Ctrl+C. The bug may be that Stop doesn't fire on cancel, or there's
a race condition.

## Events Investigated

### Stop Hook (Primary Candidate)

**Status:** Already registered, likely the correct event.

- build.rs: `"Stop"` hook → `acd claude-hook attention`
- hook-json-schema.md: Stop event has `stop_hook_active` boolean field
- install.rs: Stop hook in standard hook set

**Hypothesis:** Stop should fire on ESC, Ctrl+C, and UI stop button. But
official docs don't explicitly state what triggers Stop.

### PostToolUseFailure with is_interrupt (Secondary)

- hook-json-schema.md: PostToolUseFailure has `is_interrupt` field
- Only fires during tool execution interruption, not during thinking/response
- Too narrow — do NOT use for status transitions

### Other Events Checked

- **SubagentStop:** Only for sub-agent termination, not main session
- **SessionEnd:** Session close, not prompt cancel
- **Notification:** No cancellation-related types

## Recommendation

### Option A: Verify Stop Hook (Most Likely)

The Stop hook should handle this. Possible failure modes:

1. Stop event not firing on cancel (Claude Code behavior)
2. Hook execution race condition
3. Status update not reaching TUI

**Action:** Add debug logging, empirically test ESC and Ctrl+C.

### Option B: File Feature Request (If Stop Doesn't Fire)

If Stop does NOT fire on cancel, file a Claude Code feature request for a
`PromptCancel` or `UserInterrupt` hook event.

## Next Steps

1. Add `AGENT_CONSOLE_DASHBOARD_LOG=debug` logging to hook invocations
2. Test: prompt → wait → ESC → check logs for Stop event
3. Test: prompt → wait → Ctrl+C → check logs
4. If Stop fires but status doesn't update, investigate IPC/TUI flow
5. If Stop doesn't fire, check Claude Code GitHub for related reports
