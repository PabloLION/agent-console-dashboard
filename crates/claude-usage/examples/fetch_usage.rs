//! Example: Fetch and display Claude API usage.
//!
//! This example demonstrates how to use the claude-usage crate to retrieve
//! and display your current API usage statistics.
//!
//! # Running
//!
//! ```bash
//! cargo run -p claude-usage --example fetch_usage
//! ```
//!
//! # Prerequisites
//!
//! You must be logged into Claude Code. Run `claude` in your terminal first.

use claude_usage::{get_usage, ApiError, CredentialError, Error};

fn main() {
    println!("Claude API Usage Checker");
    println!("========================\n");

    match get_usage() {
        Ok(usage) => {
            // 5-hour usage
            println!("5-Hour Usage:");
            println!("  Utilization: {:.1}%", usage.five_hour.utilization);
            if let Some(resets_at) = usage.five_hour.resets_at {
                println!("  Resets at: {}", resets_at);
            }
            if let Some(time_left) = usage.five_hour.time_until_reset() {
                println!("  Time until reset: {} minutes", time_left.num_minutes());
            }
            match usage.five_hour_on_pace() {
                Some(true) => println!("  On pace: ✅ Yes"),
                Some(false) => println!("  On pace: ⚠️  No"),
                None => println!("  On pace: (unknown - no reset time)"),
            }

            println!();

            // 7-day usage
            println!("7-Day Usage:");
            println!("  Utilization: {:.1}%", usage.seven_day.utilization);
            if let Some(resets_at) = usage.seven_day.resets_at {
                println!("  Resets at: {}", resets_at);
            }
            if let Some(time_left) = usage.seven_day.time_until_reset() {
                println!("  Time until reset: {} hours", time_left.num_hours());
            }
            match usage.seven_day_on_pace() {
                Some(true) => println!("  On pace: ✅ Yes"),
                Some(false) => println!("  On pace: ⚠️  No"),
                None => println!("  On pace: (unknown - no reset time)"),
            }

            // Extra usage if available
            if let Some(extra) = &usage.extra_usage {
                println!();
                println!("Extra Usage:");
                println!("  Enabled: {}", if extra.is_enabled { "Yes" } else { "No" });
                if let Some(used) = extra.amount_used {
                    println!("  Amount used: ${:.2}", used);
                }
                if let Some(limit) = extra.limit {
                    println!("  Spending limit: ${:.2}", limit);
                }
            }
        }
        Err(Error::Credential(CredentialError::NotFound)) => {
            eprintln!("❌ Claude Code credentials not found.");
            eprintln!();
            eprintln!("To fix this:");
            eprintln!("  1. Install Claude Code: https://claude.ai/code");
            eprintln!("  2. Run `claude` in your terminal to login");
            eprintln!("  3. Try this command again");
            std::process::exit(1);
        }
        Err(Error::Credential(CredentialError::Expired)) => {
            eprintln!("❌ Claude Code token has expired.");
            eprintln!();
            eprintln!("Tokens are valid for ~8 hours and need periodic refresh.");
            eprintln!();
            eprintln!("To fix this:");
            eprintln!("  1. Run `claude` in your terminal");
            eprintln!("  2. The token will be automatically refreshed");
            eprintln!("  3. Try this command again");
            std::process::exit(1);
        }
        Err(Error::Api(ApiError::Unauthorized)) => {
            eprintln!("❌ Token rejected by the server.");
            eprintln!();
            eprintln!("This can happen if:");
            eprintln!("  - Your token was revoked");
            eprintln!("  - There's a clock synchronization issue");
            eprintln!("  - The token expired between local check and API call");
            eprintln!();
            eprintln!("To fix this:");
            eprintln!("  1. Run `claude` in your terminal to re-authenticate");
            eprintln!("  2. Try this command again");
            std::process::exit(1);
        }
        Err(Error::Api(ApiError::RateLimited { retry_after })) => {
            eprintln!("❌ Rate limited by the API.");
            if let Some(retry) = retry_after {
                eprintln!("   Retry after: {}", retry);
            }
            std::process::exit(1);
        }
        Err(Error::Api(ApiError::Network(msg))) => {
            eprintln!("❌ Network error: {}", msg);
            eprintln!();
            eprintln!("Check your internet connection and try again.");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            std::process::exit(1);
        }
    }
}
