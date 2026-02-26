# Research: OAuth 403 on /api/oauth/usage and Alternatives

**Date:** 2026-02-26 **Agent:** hook-research **Scope:** claude-usage crate, API
endpoints, response headers

---

## Summary

The 403 is not a blanket block on third-party callers. It is a **scope
mismatch**: ACD users who authenticated via claude.ai received a token with only
`user:inference` scope. The `/api/oauth/usage` endpoint requires `user:profile`
scope. A viable alternative exists: the `/v1/messages` response headers expose
identical utilization data and are accessible with `user:inference` scope.

---

## 1. Root Cause: Scope Mismatch, Not a Third-Party Block

### What the error says

```json
{
  "type": "error",
  "error": {
    "type": "permission_error",
    "message": "OAuth token does not meet scope requirement user:profile",
    "details": { "error_visibility": "user_facing" }
  }
}
```

HTTP status: 403

### Why our token lacks `user:profile`

Claude Code has two login flows:

```text
claude.ai login  →  inferenceOnly=true  →  scope: ["user:inference"]
console login    →  inferenceOnly=false →  scope: ["org:create_api_key",
                                                    "user:profile",
                                                    "user:inference"]
```

Source: `cli.js` (Claude Code 2.0.9), functions `BU0()` and auth flow:

```javascript
// claude.ai flow (inferenceOnly=true)
{ loginWithClaudeAi: true, inferenceOnly: true, expiresIn: 31536000 }
// scope sent to /oauth/authorize = [fH1] = ["user:inference"] only

// console flow (inferenceOnly=false)
// scope = U4A.SCOPES = ["org:create_api_key", "user:profile", "user:inference"]
```

Most users authenticate via claude.ai (the interactive UI), which uses
`inferenceOnly: true`. This gives them a long-lived token (1 year) with only
`user:inference` scope.

### Verification

Checked keychain token scopes:

```json
{ "scopes": ["user:inference"] }
```

Confirmed: our token has only `user:inference`. The `user:profile` scope is
absent.

### Can re-login fix it?

Yes — but only if the user re-authenticates through `console.anthropic.com` (not
claude.ai). GitHub issue comments on #13724 confirm "logging in again" fixes it
for users who re-authenticate via the console path.

---

## 2. Is There a Different Endpoint or Auth Method?

### Tested endpoints

```csv
Endpoint,Method,Token,Result
/api/oauth/usage,GET,user:inference Bearer,403 (user:profile required)
/api/oauth/usage,GET,No beta header,401 (OAuth not supported without beta)
/api/oauth/profile,GET,user:inference Bearer,403 (user:profile required)
/api/oauth/claude_cli/roles,GET,user:inference Bearer,403 (user:profile required)
/api/oauth/claude_cli/create_api_key,POST,user:inference Bearer,403 (org:create_api_key required)
/api/oauth/claude_cli/usage,GET,user:inference Bearer,404 (not found)
/v1/usage,GET,user:inference Bearer,404 (not found)
/api/usage,GET,user:inference Bearer,404 (not found)
/v1/messages,POST,user:inference Bearer,200 + usage headers
```

### Can API keys access /api/oauth/usage?

No. Without the `anthropic-beta: oauth-2025-04-20` header, the endpoint returns
401 "OAuth authentication is currently not supported." API key auth does not
apply to OAuth endpoints.

---

## 3. Response Headers: The Working Alternative

### Discovery

`GET /v1/messages` (inference endpoint) with a 1-token message returns usage
headers accessible with `user:inference` scope alone:

```text
anthropic-ratelimit-unified-5h-utilization: 0.72
anthropic-ratelimit-unified-5h-reset: 1772096400
anthropic-ratelimit-unified-5h-status: allowed
anthropic-ratelimit-unified-7d-utilization: 0.34
anthropic-ratelimit-unified-7d-reset: 1772463600
anthropic-ratelimit-unified-7d-status: allowed
anthropic-ratelimit-unified-status: allowed
anthropic-ratelimit-unified-representative-claim: five_hour
anthropic-ratelimit-unified-fallback: available
anthropic-ratelimit-unified-fallback-percentage: 0.5
anthropic-ratelimit-unified-overage-status: rejected
anthropic-ratelimit-unified-overage-disabled-reason: org_level_disabled
anthropic-ratelimit-unified-reset: 1772096400
```

### Data equivalence

```csv
/api/oauth/usage JSON field,Equivalent response header
five_hour.utilization,anthropic-ratelimit-unified-5h-utilization
five_hour.resets_at (ISO),anthropic-ratelimit-unified-5h-reset (Unix timestamp)
seven_day.utilization,anthropic-ratelimit-unified-7d-utilization
seven_day.resets_at (ISO),anthropic-ratelimit-unified-7d-reset (Unix timestamp)
```

The values are the same percentage scale (0.0 to 100.0+).

### This is also Claude Code's own fallback

Claude Code's `kW6()` function (cli.js) sends a 1-token "quota" message and
reads `anthropic-ratelimit-unified-*` headers to track rate limit status. It
does not parse the `5h-utilization` or `7d-utilization` headers specifically
(only `status`, `reset`, and `fallback`), but those headers are present and
available to ACD.

### Trade-off

This approach makes a real API call (1 token in, ~5 tokens out). It is not free.
The request costs approximately the minimum billable amount. However:

- This is exactly what Claude Code itself does internally
- The `user:inference` scope is all that is needed
- No re-login required for ACD users

---

## 4. Local Data Sources: None That Provide Quota Utilization

### What Claude Code stores locally

```csv
File,Content,Useful for quota?
~/.claude/stats-cache.json,Daily message/session/tool counts,No (no token counts or quota %)
~/.claude/projects/**/*.jsonl,Per-message token usage (input/output/cache),No (raw tokens not quota %)
~/.claude/statsig/*,Feature flags and experiment data,No
```

### Transcript token data

Transcripts contain per-message `usage` blocks:

```json
{
  "usage": {
    "input_tokens": 3,
    "cache_creation_input_tokens": 44574,
    "cache_read_input_tokens": 0,
    "output_tokens": 9,
    "service_tier": "standard"
  }
}
```

This gives raw token counts per message. To compute quota utilization from these
would require knowing the subscription quota limits (which are not publicly
documented and vary by plan). The API endpoint returns the derived percentage
directly, making it the only reliable source.

### Hook events

Hook stdin JSON (all events) does not contain usage or quota data. The hook JSON
schema documents: `session_id`, `cwd`, `transcript_path`, `permission_mode`,
`hook_event_name`, plus event-specific fields. None of these fields carry API
usage or quota information.

---

## 5. Third-Party Tool Approaches

### GitHub issues

Two active GitHub issues on anthropics/claude-code confirm this is a widespread
problem:

- **#13724** "OAuth token missing user:profile scope for usage data" (open,
  Dec 2025) — confirmed affects claude.ai login users; workaround is re-login
  via console
- **#11985** "OAuth token missing user:profile scope" (open, Nov 2025) — same
  root cause on Linux

### claude-usage npm package

The `claude-usage` npm package (v1.1.0 on npm, for analytics dashboard) uses a
different approach altogether — it parses local Claude Code sqlite databases and
transcript files for usage statistics rather than calling the API.

### OpenClaw and similar tools

No public information found on how they handle the scope requirement. The scope
limitation appears to be a relatively new enforcement as of late 2025.

---

## 6. Recommended Approach

### Approach A (Recommended): Parse response headers from `/v1/messages`

Send a 1-token inference request with
`messages: [{"role": "user", "content": "quota"}]` and parse the
`anthropic-ratelimit-unified-5h-utilization`, `5h-reset`, `7d-utilization`,
`7d-reset` response headers.

Pros:

- Works for all users regardless of login method
- Same approach Claude Code uses internally
- No re-login required
- `user:inference` scope is sufficient

Cons:

- Makes a real API call (minimum token cost, approximately negligible)
- Requires network access
- Adds one network round-trip to the startup sequence

### Approach B: Prompt user to re-login via console

Display an error when 403 occurs, directing users to run
`claude logout && claude login` choosing the console.anthropic.com path.

Pros:

- No code changes to the API call
- Permanent fix for the user's token

Cons:

- Poor UX — user action required
- Many users don't know the difference between login methods
- Token still expires every 8 hours (though auto-refresh preserves scope)

### Approach C: Degrade gracefully (show "N/A")

On 403 from `/api/oauth/usage`, display "N/A" or hide the usage widget.

Pros:

- Simplest code change
- No API costs

Cons:

- Loses all usage data for claude.ai users
- No actionable path for the user

---

## 7. Required Code Changes for Approach A

### Changes to `crates/claude-usage/src/client.rs`

Add a new function `fetch_usage_from_headers()` that:

1. Sends `POST /v1/messages` with `model: <any>`, `max_tokens: 1`,
   `messages: [{"role":"user","content":"quota"}]`
2. Reads response headers (not body)
3. Parses `anthropic-ratelimit-unified-5h-utilization` and `7d-utilization` as
   `f64`
4. Parses `5h-reset` and `7d-reset` as Unix timestamps, converts to
   `DateTime<Utc>`
5. Returns `UsageData`

### Changes to `crates/claude-usage/src/lib.rs`

The `get_usage()` function should try `/api/oauth/usage` first (for
`user:profile` tokens), fall back to the headers approach on 403.

### Changes to `crates/claude-usage/src/error.rs`

Add `ApiError::InsufficientScope` (maps from 403) to enable clean fallback
logic.

### Changes to `crates/claude-usage/Cargo.toml`

The existing `reqwest` dependency already supports reading response headers. No
new dependencies needed.

### Model selection for the probe request

Use the cheapest available model. Looking at the Claude Code source, `kW6()`
uses `Mz()` (the currently selected model). For ACD's probe, use
`claude-haiku-4-5` as a hardcoded cheap model, or read from the token's
inference preferences. A 1-token request costs fractions of a cent.

---

## 8. What the 403 Response Body Says Exactly

```json
{
  "type": "error",
  "error": {
    "type": "permission_error",
    "message": "OAuth token does not meet scope requirement user:profile",
    "details": { "error_visibility": "user_facing" }
  },
  "request_id": "req_011CYWHeotsdXUJ8o3monhnT"
}
```

HTTP 403. Not rate limiting. Not a blanket third-party block. Purely a scope
enforcement issue.

---

## Sources

- Claude Code source:
  `/opt/homebrew/lib/node_modules/@anthropic-ai/claude-code/cli.js` (v2.0.9) —
  functions `$X()`, `Xr2()`, `kW6()`, `vn2()`, `OH0()`, `BU0()`
- macOS Keychain: token scopes confirmed via `security find-generic-password`
- Live API tests: all curl experiments above run with real token 2026-02-26
- [GitHub issue #13724](https://github.com/anthropics/claude-code/issues/13724)
- [GitHub issue #11985](https://github.com/anthropics/claude-code/issues/11985)
