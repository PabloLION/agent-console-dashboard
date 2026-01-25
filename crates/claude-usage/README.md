# claude-usage

Fetch Claude API usage data from Anthropic.

A Rust library that retrieves usage statistics for Claude API, including 5-hour
and 7-day utilization percentages. Available for both Rust (crates.io) and
Node.js (npm via napi-rs).

## Features

- Cross-platform credential retrieval (macOS Keychain, Linux credential file)
- Typed response structures for usage data
- Secure credential handling (read, use, discard immediately)
- Helper methods for utilization analysis (on-pace detection, time until reset)
- Node.js bindings via napi-rs

## Platform Support

| Platform | Credential Source                    | Status |
| -------- | ------------------------------------ | ------ |
| macOS    | Keychain ("Claude Code-credentials") | ✅     |
| Linux    | `~/.claude/.credentials.json`        | ✅     |
| Windows  | -                                    | ❌     |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
claude-usage = "0.1"
```

Or install from npm:

```bash
npm install claude-usage
# or
yarn add claude-usage
```

## Quick Start

### Rust Example

```rust
use claude_usage::get_usage;

fn main() -> Result<(), claude_usage::Error> {
    let usage = get_usage()?;

    println!("5-hour utilization: {}%", usage.five_hour.utilization);
    println!("7-day utilization: {}%", usage.seven_day.utilization);

    // Check if usage is sustainable
    if usage.five_hour_on_pace() {
        println!("5-hour usage is on pace");
    }

    // Get time until reset
    let time_left = usage.five_hour.time_until_reset();
    println!("Resets in {} minutes", time_left.num_minutes());

    Ok(())
}
```

### Node.js / TypeScript

```typescript
const { getUsage, isOnPace } = require("claude-usage");

try {
  const usage = getUsage();

  console.log(`5-hour utilization: ${usage.fiveHour.utilization}%`);
  console.log(`7-day utilization: ${usage.sevenDay.utilization}%`);

  // Check if usage is sustainable
  const onPace = isOnPace(
    usage.fiveHour.utilization,
    usage.fiveHour.resetsAt,
    5,
  );
  console.log(`On pace: ${onPace}`);
} catch (error) {
  console.error("Failed to fetch usage:", error.message);
}
```

## API Reference

### Rust API

#### `get_usage() -> Result<UsageData, Error>`

Main entry point. Fetches current usage data from the Anthropic API.

```rust
let usage = claude_usage::get_usage()?;
```

#### `UsageData`

```rust
pub struct UsageData {
    pub five_hour: UsagePeriod,        // 5-hour rolling window
    pub seven_day: UsagePeriod,        // 7-day rolling window
    pub seven_day_sonnet: Option<UsagePeriod>,  // Sonnet-specific (if applicable)
    pub extra_usage: Option<ExtraUsage>,        // Billing info (if enabled)
}
```

#### `UsagePeriod`

```rust
pub struct UsagePeriod {
    pub utilization: f64,              // Percentage (0.0 - 100.0+)
    pub resets_at: DateTime<Utc>,      // When quota resets
}

impl UsagePeriod {
    fn time_until_reset(&self) -> TimeDelta;
    fn time_elapsed_percent(&self, period_hours: u32) -> f64;
    fn is_on_pace(&self, period_hours: u32) -> bool;
}
```

#### Helper Methods

```rust
// Check if usage is sustainable
usage.five_hour_on_pace()   // true if 5h usage won't exceed quota
usage.seven_day_on_pace()   // true if 7d usage won't exceed quota

// Time calculations
usage.five_hour.time_until_reset()        // Duration until reset
usage.five_hour.time_elapsed_percent(5)   // % of period elapsed
usage.five_hour.is_on_pace(5)             // Manual on-pace check
```

### Node.js API

#### `getUsage(): UsageData`

Synchronously fetches current usage data. Throws on error.

#### `isOnPace(utilization: number, resetsAt: string, periodHours: number): boolean`

Checks if current utilization is sustainable for the given period.

### Error Types

| Error          | Cause                              | Solution                      |
| -------------- | ---------------------------------- | ----------------------------- |
| `NotFound`     | Credentials not in secure storage  | Run `claude` to login         |
| `Expired`      | Token has expired                  | Run `claude` to re-login      |
| `Unauthorized` | API rejected token                 | Run `claude` to re-login      |
| `RateLimited`  | Too many requests                  | Wait for retry-after period   |
| `Network`      | Connection failed                  | Check internet connection     |
| `Parse`        | Invalid credential/response format | Re-login or report bug        |
| `Permission`   | Cannot read credential file        | Check file permissions        |
| `NoHomeDir`    | HOME environment variable not set  | Set HOME environment variable |

## Environment Variables

| Variable                  | Description                                     |
| ------------------------- | ----------------------------------------------- |
| `CLAUDE_CODE_OAUTH_TOKEN` | Override file-based credentials (all platforms) |

## Security

This crate follows strict security practices for credential handling:

1. **Read-and-discard**: Tokens are read from secure storage, used for a single
   API call, and immediately discarded
2. **No storage**: Tokens are never stored in memory, files, or logs
3. **No propagation**: Tokens are never passed to other modules or functions
4. **Generic errors**: Error messages never include credential data
5. **Platform security**: Uses OS-native secure storage (Keychain on macOS,
   credential file on Linux)

### Credential Flow

```text
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Secure Storage │────▶│  HTTP Request   │────▶│  Token Discarded│
│  (Keychain/File)│     │  (Bearer Auth)  │     │  (Out of Scope) │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        │                       │
        └───────────────────────┴──── Token lifetime: single function call
```

## Architecture

```text
claude-usage/
├── src/
│   ├── lib.rs           # Public API: get_usage()
│   ├── client.rs        # HTTP client for Anthropic API
│   ├── credentials/     # Platform-specific credential retrieval
│   │   ├── mod.rs       # Shared logic and get_token()
│   │   ├── macos.rs     # Keychain integration
│   │   └── linux.rs     # Credential file reading
│   ├── types.rs         # UsageData, UsagePeriod, ExtraUsage
│   ├── error.rs         # Error types
│   └── napi.rs          # Node.js bindings (optional)
├── Cargo.toml
└── README.md
```

### Feature Flags

| Feature    | Description                    | Default |
| ---------- | ------------------------------ | ------- |
| `blocking` | Enable synchronous HTTP client | ✅      |
| `napi`     | Enable Node.js bindings        | ❌      |

## Troubleshooting

### "Credentials not found"

Claude Code must be installed and logged in:

```bash
# Install Claude Code (if not installed)
# See: https://docs.anthropic.com/claude-code

# Login to Claude Code
claude
```

### "Credentials expired"

Re-authenticate with Claude Code:

```bash
claude
```

### "Permission denied"

On Linux, check credential file permissions:

```bash
ls -la ~/.claude/.credentials.json
# Should be readable by your user
```

### Testing without credentials

Use the environment variable to provide a token directly:

```bash
export CLAUDE_CODE_OAUTH_TOKEN="sk-ant-oat01-..."
```

### Rate limiting

If you receive `RateLimited` errors, wait for the `retry_after` period. Consider
caching usage data in your application to reduce API calls.

## Requirements

- Rust 1.77+ (for building)
- Claude Code installed and logged in
- Valid OAuth credentials in platform-specific storage

## API Endpoint

This crate calls the Anthropic OAuth usage endpoint:

```text
GET https://api.anthropic.com/api/oauth/usage
Authorization: Bearer <token>
anthropic-beta: oauth-2025-04-20
```

### Response Structure

```json
{
  "five_hour": {
    "utilization": 8.0,
    "resets_at": "2026-01-22T09:00:00Z"
  },
  "seven_day": {
    "utilization": 77.0,
    "resets_at": "2026-01-22T19:00:00Z"
  },
  "seven_day_sonnet": {
    "utilization": 0.0,
    "resets_at": "2026-01-25T00:00:00Z"
  },
  "extra_usage": {
    "is_enabled": false
  }
}
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

## Related

- [agent-console-dashboard][acd] - TUI dashboard for Claude Code (parent
  project)
- [Claude Code](https://docs.anthropic.com/claude-code) - Anthropic's coding
  assistant

[acd]: https://github.com/PabloLION/agent-console-dashboard
