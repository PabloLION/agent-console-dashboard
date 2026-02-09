//! # claude-usage
//!
//! A library for fetching Claude API usage data from Anthropic.
//!
//! This crate provides a simple API to retrieve usage statistics including
//! 5-hour and 7-day utilization percentages. It handles credential retrieval
//! from platform-specific secure storage and returns typed, structured data.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use claude_usage::get_usage;
//!
//! let usage = get_usage()?;
//! println!("5h utilization: {}%", usage.five_hour.utilization);
//! println!("7d utilization: {}%", usage.seven_day.utilization);
//! ```
//!
//! ## Features
//!
//! - **Cross-platform credentials**: macOS Keychain, Linux credential file
//! - **Typed responses**: [`UsageData`], [`UsagePeriod`], [`ExtraUsage`]
//! - **Secure handling**: Tokens are read, used, and immediately discarded
//! - **Helper methods**: Check if usage is on-pace, time until reset
//! - **Node.js bindings**: Available via the `napi` feature
//!
//! ## Platform Support
//!
//! | Platform | Credential Source | Status |
//! |----------|-------------------|--------|
//! | macOS | Keychain ("Claude Code-credentials") | ✅ |
//! | Linux | `~/.claude/.credentials.json` | ✅ |
//! | Windows | - | ❌ |
//!
//! ## Usage Examples
//!
//! ### Basic Usage
//!
//! ```rust,ignore
//! use claude_usage::get_usage;
//!
//! fn main() -> Result<(), claude_usage::Error> {
//!     let usage = get_usage()?;
//!
//!     println!("5-hour: {}%", usage.five_hour.utilization);
//!     println!("7-day: {}%", usage.seven_day.utilization);
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Checking If Usage Is Sustainable
//!
//! ```rust,ignore
//! use claude_usage::get_usage;
//!
//! let usage = get_usage()?;
//!
//! // Check if current usage rate won't exceed quota before reset
//! if usage.five_hour_on_pace() {
//!     println!("5-hour usage is sustainable");
//! } else {
//!     println!("Warning: 5-hour usage may exceed quota!");
//! }
//!
//! if usage.seven_day_on_pace() {
//!     println!("7-day usage is sustainable");
//! }
//! ```
//!
//! ### Time Until Reset
//!
//! ```rust,ignore
//! use claude_usage::get_usage;
//!
//! let usage = get_usage()?;
//!
//! let time_left = usage.five_hour.time_until_reset();
//! println!("5-hour quota resets in {} minutes", time_left.num_minutes());
//!
//! // Get percentage of period elapsed
//! let elapsed = usage.five_hour.time_elapsed_percent(5);
//! println!("{}% of 5-hour period has elapsed", elapsed);
//! ```
//!
//! ### Handling Errors
//!
//! ```rust,ignore
//! use claude_usage::{get_usage, Error, CredentialError, ApiError};
//!
//! match get_usage() {
//!     Ok(usage) => println!("Usage: {}%", usage.five_hour.utilization),
//!     Err(Error::Credential(CredentialError::NotFound)) => {
//!         eprintln!("Please run `claude` to login first");
//!     }
//!     Err(Error::Credential(CredentialError::Expired)) => {
//!         eprintln!("Token expired. Please run `claude` to re-login");
//!     }
//!     Err(Error::Api(ApiError::RateLimited { retry_after })) => {
//!         eprintln!("Rate limited. Retry after: {:?}", retry_after);
//!     }
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! ```
//!
//! ## Environment Variable
//!
//! The `CLAUDE_CODE_OAUTH_TOKEN` environment variable can override
//! file-based credential retrieval on any platform:
//!
//! ```bash
//! export CLAUDE_CODE_OAUTH_TOKEN="sk-ant-oat01-..."
//! ```
//!
//! ## Module Overview
//!
//! - [`client`]: HTTP client for the Anthropic usage API
//! - [`credentials`]: Platform-specific credential retrieval
//! - [`types`]: Response types ([`UsageData`], [`UsagePeriod`], [`ExtraUsage`])
//! - [`error`]: Error types ([`Error`], [`CredentialError`], [`ApiError`])
//! - `napi`: Node.js bindings (requires `napi` feature)
//!
//! ## Security
//!
//! This crate follows strict security practices:
//!
//! 1. Tokens are read from secure storage, used once, and immediately discarded
//! 2. Tokens are never stored in memory, logged, or passed to other modules
//! 3. Error messages use generic text to prevent credential exposure

pub mod client;
pub mod credentials;
pub mod error;
#[cfg(feature = "napi")]
pub mod napi;
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
    fn env__get_usage() {
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
