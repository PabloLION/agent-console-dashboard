# Decision: Credential Storage

**Decided:** 2026-01-22 **Status:** Implemented

## Context

The API usage widget needs OAuth credentials to call the Anthropic usage
endpoint. Rather than requiring a separate API key, the project reuses Claude
Code's own OAuth credentials, which are stored differently on each platform.

## Decision

Reuse Claude Code's OAuth credentials with platform-specific retrieval. The
`claude-usage` crate (published to crates.io) handles all credential retrieval
and API calls.

| Platform | Storage   | Location                      | Access Method            |
| -------- | --------- | ----------------------------- | ------------------------ |
| macOS    | Keychain  | `"Claude Code-credentials"`   | security-framework crate |
| Linux    | JSON file | `~/.claude/.credentials.json` | File read + JSON parse   |

Environment variable `CLAUDE_CODE_OAUTH_TOKEN` takes precedence if set.

### API Endpoint

```text
GET https://api.anthropic.com/api/oauth/usage
Authorization: Bearer <token>
anthropic-beta: oauth-2025-04-20
```

## Rationale

- No separate API key required (zero user setup for usage tracking)
- Credential format is the same on both platforms (JSON with `claudeAiOauth`)
- Linux storage is less secure than macOS Keychain (plain file), but follows
  Claude Code's own convention

## Security Requirements

- Isolate credential handling to a single module
- Read token, make API call, discard token immediately
- Never store token in memory longer than needed
- Never pass token to other modules (only pass the API response data)
- Never log or serialize the token

## Error Handling

| Condition      | Behavior                                 |
| -------------- | ---------------------------------------- |
| Token expired  | Check `expiresAt` timestamp before use   |
| Token invalid  | API returns 401, show "Re-login" message |
| No credentials | Show "Claude Code not logged in" message |

## Implementation

Implemented in E011 (Claude Usage Crate), published as `claude-usage` on
crates.io.

[Q34](../archive/planning/6-open-questions.md) |
[Q72](../archive/planning/6-open-questions.md)
