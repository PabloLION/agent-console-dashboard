# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.2] - 2026-01-26

### Changed

- Example: Replace "on-pace" indicator with direct percentage comparison
  (`Quota used: 41%  |  Time elapsed: 75%`) for easier interpretation

## [0.2.1] - 2026-01-26

### Changed

- macOS: Use `/usr/bin/security` CLI instead of `security-framework` crate for
  Keychain access. This eliminates password prompts since the CLI is already
  authorized in Claude Code's Keychain ACL.

### Removed

- Removed `security-framework` dependency (macOS-only). Now uses standard
  library `std::process::Command` to call the system `security` binary.

## [0.2.0] - 2026-01-26

### Changed

- **Breaking**: `time_until_reset()`, `time_elapsed_percent()`, `is_on_pace()`
  now return `Option<T>` to handle cases where `resets_at` is unavailable
- **Breaking**: `five_hour_on_pace()`, `seven_day_on_pace()` now return
  `Option<bool>`
- `UsagePeriod.resets_at` is now `Option<DateTime<Utc>>` to handle null API
  responses
- Improved error messages to use generic text instead of potentially exposing
  sensitive data
- Use `chrono` for timestamp calculations instead of manual UNIX epoch math
- Node.js `isOnPace` function now returns `Result` instead of silently returning
  `false` on parse errors

### Fixed

- macOS Keychain lookup now uses current username instead of empty string
- API response parsing no longer fails when `resets_at` is null
- Test cleanup now uses RAII pattern to prevent environment pollution on panic

### Added

- `examples/fetch_usage.rs` for end-to-end testing and usage demonstration
- Documentation for 8-hour OAuth token lifecycle and refresh behavior

## [0.1.0] - 2026-01-22

### Added

- Initial release
- `get_usage()` function to fetch Claude API usage data
- macOS Keychain credential retrieval
- Linux credential file retrieval (`~/.claude/.credentials.json`)
- `UsageData`, `UsagePeriod`, `ExtraUsage` typed response structs
- Helper methods: `five_hour_on_pace()`, `seven_day_on_pace()`
- Time utilities: `time_until_reset()`, `time_elapsed_percent()`, `is_on_pace()`
- Environment variable override: `CLAUDE_CODE_OAUTH_TOKEN`
- Node.js bindings via napi-rs (`napi` feature)
- Comprehensive error types: `CredentialError`, `ApiError`, `Error`

### Security

- Tokens are read and immediately discarded after use
- Generic error messages prevent credential exposure
- Uses platform-native secure storage

[Unreleased]:
  https://github.com/PabloLION/agent-console-dashboard/compare/claude-usage-v0.2.2...HEAD
[0.2.2]:
  https://github.com/PabloLION/agent-console-dashboard/compare/claude-usage-v0.2.1...claude-usage-v0.2.2
[0.2.1]:
  https://github.com/PabloLION/agent-console-dashboard/compare/claude-usage-v0.2.0...claude-usage-v0.2.1
[0.2.0]:
  https://github.com/PabloLION/agent-console-dashboard/compare/claude-usage-v0.1.0...claude-usage-v0.2.0
[0.1.0]:
  https://github.com/PabloLION/agent-console-dashboard/releases/tag/claude-usage-v0.1.0
