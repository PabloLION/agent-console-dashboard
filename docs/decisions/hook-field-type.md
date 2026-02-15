# Hook Config Fields: `Option<String>` over Empty String

Created: 20260215T030000Z Issue: acd-1j2

## Problem

Hook config fields (`activate_hook`, `reopen_hook`) need a way to represent "no
hook configured". Two options:

- `String` where empty string means no hook
- `Option<String>` where `None` means no hook

## Decision

Use `Option<String>` for both hook fields.

### Why

`None` means "not configured" unambiguously. An empty string is a sentinel value
— every consumer must know that empty means none. `Option` makes the intent
self-documenting at the type level.

This required refactoring `activate_hook` (renamed from `double_click_hook`)
from `String` to `Option<String>`. The config schema uses serde's default `None`
for absent TOML keys.

### Trade-off

The refactoring cost is small: change the field type, update deserialization,
and replace `is_empty()` checks with `is_some()`. The user's preference was
explicit: "go with the correct option, not the easy option."

## Orchestration Rule Update

Also in this session: the pre-dispatch protocol was updated to require showing
every issue to the user before dispatch, even when the orchestrator has zero
doubts. The user may have doubts of their own. This is documented in
`.claude/rules/orchestration.md`.
