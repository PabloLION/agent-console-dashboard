# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Improved error messages to use generic text instead of potentially exposing
  sensitive data
- Use `chrono` for timestamp calculations instead of manual UNIX epoch math
- Node.js `isOnPace` function now returns `Result` instead of silently returning
  `false` on parse errors

### Fixed

- Test cleanup now uses RAII pattern to prevent environment pollution on panic

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
  https://github.com/PabloLION/agent-console-dashboard/compare/claude-usage-v0.1.0...HEAD
[0.1.0]:
  https://github.com/PabloLION/agent-console-dashboard/releases/tag/claude-usage-v0.1.0
