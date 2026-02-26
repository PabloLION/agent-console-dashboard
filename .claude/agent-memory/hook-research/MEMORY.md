# Hook Research Agent Memory

## Key Findings

### OAuth /api/oauth/usage 403 (acd-research 2026-02-26)

Root cause: scope mismatch, NOT a third-party block.

- claude.ai login → `inferenceOnly=true` → only `user:inference` scope
- console.anthropic.com login → full scopes incl. `user:profile`
- `/api/oauth/usage` requires `user:profile` scope → 403 for claude.ai users

**Working alternative**: `POST /v1/messages` (1-token probe) returns headers:

- `anthropic-ratelimit-unified-5h-utilization` (float, same scale as API)
- `anthropic-ratelimit-unified-5h-reset` (Unix timestamp)
- `anthropic-ratelimit-unified-7d-utilization` (float)
- `anthropic-ratelimit-unified-7d-reset` (Unix timestamp)

These headers work with `user:inference` scope. This is Claude Code's own
internal fallback mechanism (function `kW6()` in cli.js). Claude Code does NOT
parse the 5h/7d utilization headers itself (only generic status/reset), but they
are present in every inference response.

Detailed report: `docs/agents/hook-research/oauth-403-research.md`

### Hook JSON Schema

Documented in `.git-ignored/hook-json-schema.md`. Hook stdin does NOT contain
API usage or quota data. Only: session_id, cwd, transcript_path,
permission_mode, hook_event_name, plus event-specific fields.

### Local File Sources for Usage Data

- `~/.claude/stats-cache.json`: daily message/session/tool counts (no quota %)
- `~/.claude/projects/**/*.jsonl`: per-message token counts (no quota %)
- No local file stores quota utilization percentages

### Transcript Token Data

Transcripts have per-message `usage.input_tokens`/`output_tokens`/`cache_*` but
NOT quota percentages. Quota % requires knowing subscription limits.
