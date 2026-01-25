# Epic: Claude Usage Crate

**Epic ID:** E011 **Status:** Draft **Priority:** High **Estimated Effort:** M

## Summary

Create a standalone, cross-platform Rust crate (`claude-usage`) that fetches API
usage data from Anthropic's OAuth usage endpoint. This crate handles credential
retrieval from platform-specific secure storage and returns typed, structured
usage data. Published to crates.io and npm (via napi-rs) for ecosystem reuse.

## Goals

- Provide a simple API: `get_usage() -> Result<UsageData>`
- Support macOS (Keychain) and Linux (credential file) platforms
- Return typed response with 5h/7d utilization percentages
- Publish to crates.io to register the `claude-usage` name
- Provide npm package via napi-rs for Node.js consumers
- Isolate credential handling with strict security practices

## User Value

Developers building Claude Code tooling need a reliable way to fetch usage data.
Currently, no simple cross-platform library exists that handles both credential
retrieval and API calls. This crate fills that gap, enabling:

- Dashboard apps to display usage
- CLI tools to check quotas
- Monitoring integrations
- Any tool that needs Claude usage visibility

## Stories

| Story ID                                                | Title                           | Priority | Status |
| ------------------------------------------------------- | ------------------------------- | -------- | ------ |
| [S040](../stories/S040-workspace-restructure.md)        | Restructure as Cargo workspace  | P0       | Draft  |
| [S041](../stories/S041-macos-credential-fetch.md)       | macOS Keychain credential fetch | P0       | Draft  |
| [S042](../stories/S042-linux-credential-fetch.md)       | Linux credential file fetch     | P1       | Draft  |
| [S043](../stories/S043-usage-api-client.md)             | Usage API client                | P0       | Draft  |
| [S044](../stories/S044-typed-usage-response.md)         | Typed usage response structs    | P0       | Draft  |
| [S045](../stories/S045-publish-crates-io.md)            | Publish to crates.io            | P0       | Draft  |
| [S046](../stories/S046-napi-rs-bindings.md)             | napi-rs bindings for npm        | P2       | Draft  |
| [S047](../stories/S047-update-e009-use-claude-usage.md) | Update E009 to use claude-usage | P2       | Draft  |

## Dependencies

- None (standalone crate, but will be consumed by E009)

## Acceptance Criteria

- [ ] Crate compiles and tests pass on macOS and Linux
- [ ] `get_usage()` returns structured data with 5h/7d utilization
- [ ] Credentials are read and immediately discarded (never stored in memory)
- [ ] Published to crates.io as `claude-usage`
- [ ] npm package available via napi-rs
- [ ] E009 updated to consume this crate instead of duplicating logic

## Technical Notes

### API Endpoint

```text
GET https://api.anthropic.com/api/oauth/usage
Authorization: Bearer <token>
anthropic-beta: oauth-2025-04-20
```

### Response Structure

```json
{
  "five_hour": { "utilization": 8.0, "resets_at": "2026-01-22T09:00:00Z" },
  "seven_day": { "utilization": 77.0, "resets_at": "2026-01-22T19:00:00Z" },
  "seven_day_sonnet": { "utilization": 0.0, "resets_at": "..." },
  "extra_usage": { "is_enabled": false, ... }
}
```

### Credential Sources

| Platform | Storage   | Location                      |
| -------- | --------- | ----------------------------- |
| macOS    | Keychain  | `"Claude Code-credentials"`   |
| Linux    | JSON file | `~/.claude/.credentials.json` |

### Security Requirements

- Read token → make API call → discard token immediately
- Never store token in memory longer than needed
- Never pass token to other modules
- Never log or serialize the token
- Isolate credential handling to single module

### Workspace Structure

```text
agent-console-dashboard/
├── Cargo.toml                    # workspace root
├── crates/
│   ├── agent-console-dashboard/  # binary crate (existing code)
│   └── claude-usage/             # library crate (new)
```

### Publishing Roadmap

| Version | Registry  | Method  |
| ------- | --------- | ------- |
| v0.1    | crates.io | Rust    |
| v0.2+   | npm       | napi-rs |

## Out of Scope

- Windows support (deferred to v2+)
- Token refresh logic (Claude Code handles this)
- Caching of usage data (caller's responsibility)
- Historical usage tracking
