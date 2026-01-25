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
//! ## Example
//!
//! ```rust,ignore
//! use claude_usage::get_usage;
//!
//! let usage = get_usage()?;
//! println!("5h utilization: {}%", usage.five_hour.utilization);
//! println!("7d utilization: {}%", usage.seven_day.utilization);
//! ```

// Placeholder - functionality to be added in subsequent stories
