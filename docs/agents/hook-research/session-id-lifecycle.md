# Session ID Lifecycle Across /clear, /compact, /resume

Research for acd-5a1e. Conducted: 2026-02-16.

## Summary

**session_id is NOT stable.** Every SessionStart event (startup, resume, clear,
compact) generates a new UUID. Our previous assumption of stability was wrong.

## Findings

### 1. Does /clear generate a new session_id?

**Yes.** User observed two different session IDs for the same working directory.
/clear fires a SessionStart event with `source: "clear"`, which generates a new
UUID.

### 2. Does /compact preserve the session_id?

**No (very likely).** /compact fires a SessionStart event with
`source: "compact"`, same mechanism as /clear. No evidence of special handling
to preserve the UUID.

### 3. Does /resume preserve the session_id?

**No.** Confirmed by [GitHub issue #12235][gh-12235] (closed as duplicate).
When resuming with `--resume`, hooks receive a new UUID, not the original
session's ID.

The issue author requested an `original_session_id` or `resumed_from` field to
link sessions. As of issue closure, no such field exists.

## References

- [GitHub issue #12235: Session ID changes when resuming][gh-12235] — direct
  confirmation that --resume generates new UUID. Closed as duplicate (known
  behavior). Reporter: arunsathiya, Claude Code v2.0.50.
- [Claude Code hooks reference][hooks-ref] — SessionStart matchers document
  four sources: `startup`, `resume`, `clear`, `compact`. Each fires a new
  SessionStart event. The docs do not explicitly state whether session_id
  changes, but the GitHub issue confirms it does.
- `.git-ignored/hook-json-schema.md` lines 18-19 — our local doc claims
  "Stable across resume, clear, and compact within the same session." **This
  claim is incorrect** and should be updated.

## Implications for ACD

1. **Orphaned sessions**: /clear, /compact, and /resume all create new session
   entries in the daemon. The old session never receives new hooks — it stays
   in its last state forever (eventually dimmed as inactive).
2. **Same directory, multiple sessions**: A single working directory can have
   multiple session entries (one per /clear or /resume). The daemon shows all
   of them.
3. **Garbage collection needed**: Without cleanup, the session list grows
   unboundedly. Options: auto-remove inactive sessions for the same directory
   when a new session starts, or add a manual cleanup command.
4. **hook-json-schema.md needs correction**: Remove the stability claim.

[gh-12235]: https://github.com/anthropics/claude-code/issues/12235
[hooks-ref]: https://code.claude.com/docs/en/hooks
