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
//! ```
//!
//! ## Environment Variable
//!
//! The `CLAUDE_CODE_OAUTH_TOKEN` environment variable can be set to override
//! file-based credential retrieval on any platform.

pub mod client;
pub mod credentials;
pub mod error;

#[cfg(feature = "blocking")]
pub use client::fetch_usage_raw;
pub use credentials::get_token;
pub use error::{ApiError, CredentialError};
