//! # claude-usage
//!
//! A library for fetching Claude API usage data from Anthropic.
//!
//! This crate provides a simple API to retrieve usage statistics including
//! 5-hour and 7-day utilization percentages.
//!
//! ## Features
//!
//! - Cross-platform credential retrieval (macOS Keychain, Linux credential file)
//! - Typed response structures for usage data
//! - Secure credential handling (read, use, discard immediately)
//!
//! ## Platform Support
//!
//! | Platform | Credential Source |
//! |----------|-------------------|
//! | macOS | Keychain ("Claude Code-credentials") |
//! | Linux | `~/.claude/.credentials.json` |
//!
//! ## Example
//!
//! ```rust,ignore
//! use claude_usage::get_usage;
//!
//! let usage = get_usage()?;
//! println!("5h utilization: {}%", usage.five_hour.utilization);
//! println!("7d utilization: {}%", usage.seven_day.utilization);
//!
//! // Check if usage is sustainable
//! if usage.five_hour_on_pace() {
//!     println!("5-hour usage is on pace");
//! }
//! ```
//!
//! ## Environment Variable
//!
//! The `CLAUDE_CODE_OAUTH_TOKEN` environment variable can be set to override
//! file-based credential retrieval on any platform.

pub mod client;
pub mod credentials;
pub mod error;
pub mod types;

#[cfg(feature = "blocking")]
pub use client::fetch_usage_raw;
pub use credentials::get_token;
pub use error::{ApiError, CredentialError, Error};
pub use types::{ExtraUsage, UsageData, UsagePeriod};

/// Fetch current Claude API usage data.
///
/// This is the main entry point for the crate. It:
/// 1. Retrieves credentials from platform-specific storage
/// 2. Calls the Anthropic usage API
/// 3. Returns typed usage data
///
/// # Example
///
/// ```rust,ignore
/// use claude_usage::get_usage;
///
/// let usage = get_usage()?;
/// println!("5h utilization: {}%", usage.five_hour.utilization);
/// println!("7d utilization: {}%", usage.seven_day.utilization);
/// ```
///
/// # Errors
///
/// Returns [`Error`] if:
/// - Credentials are not found or expired
/// - API call fails
/// - Response parsing fails
#[cfg(feature = "blocking")]
pub fn get_usage() -> Result<UsageData, Error> {
    let token = credentials::get_token()?;
    let response = client::fetch_usage_raw(&token)?;
    let usage: UsageData =
        serde_json::from_str(&response).map_err(|e| Error::Parse(e.to_string()))?;
    Ok(usage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires real credentials"]
    #[cfg(feature = "blocking")]
    fn test_get_usage_integration() {
        let result = get_usage();
        match result {
            Ok(usage) => {
                println!("5h utilization: {}%", usage.five_hour.utilization);
                println!("7d utilization: {}%", usage.seven_day.utilization);
                assert!(usage.five_hour.utilization >= 0.0);
                assert!(usage.seven_day.utilization >= 0.0);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}
