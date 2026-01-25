# claude-usage

Fetch Claude API usage data from Anthropic.

This crate provides a simple API to retrieve usage statistics for Claude API,
including 5-hour and 7-day utilization percentages.

## Features

- Cross-platform credential retrieval (macOS Keychain, Linux credential file)
- Typed response structures for usage data
- Secure credential handling (read, use, discard immediately)
- Helper methods for utilization analysis

## Platform Support

| Platform | Credential Source                    |
| -------- | ------------------------------------ |
| macOS    | Keychain ("Claude Code-credentials") |
| Linux    | `~/.claude/.credentials.json`        |

## Installation

```toml
[dependencies]
claude-usage = "0.1"
```

## Usage

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

## Environment Variable

The `CLAUDE_CODE_OAUTH_TOKEN` environment variable can be set to override
file-based credential retrieval on any platform.

## Requirements

- Claude Code must be installed and logged in
- Valid OAuth credentials in platform-specific storage

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
