//! macOS Keychain credential retrieval.
//!
//! This module retrieves Claude Code OAuth credentials from the macOS Keychain.
//! The credentials are stored by Claude Code under the service name
//! "Claude Code-credentials".
//!
//! # Implementation Note
//!
//! We use the `/usr/bin/security` CLI command instead of the `security-framework`
//! crate's direct API calls. This is because:
//!
//! - Claude Code adds `/usr/bin/security` to the Keychain item's ACL (Access Control List)
//! - Direct API calls via `SecItemCopyMatching` use our binary as the requester
//! - Our binary is NOT in the ACL, so macOS would prompt for password
//! - By shelling out to `/usr/bin/security`, we use an already-authorized binary
//!
//! This approach mirrors how the Swift-based Claude Usage Tracker handles this.

use std::process::Command;

use super::{parse_credential_json, KEYCHAIN_SERVICE};
use crate::error::CredentialError;

/// Retrieve the OAuth access token from macOS Keychain.
///
/// Uses the `/usr/bin/security` CLI command to avoid password prompts.
/// The `security` binary is already in Claude Code's Keychain ACL.
///
/// # Errors
///
/// Returns [`CredentialError`] if:
/// - Credentials are not found in Keychain
/// - Credentials cannot be parsed
/// - Token is expired
pub fn get_token_macos() -> Result<String, CredentialError> {
    let username = get_current_username()?;

    // Use /usr/bin/security CLI - it's already authorized in the ACL
    let output = Command::new("/usr/bin/security")
        .args([
            "find-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            &username,
            "-w", // Print password only (no metadata)
        ])
        .output()
        .map_err(|_| CredentialError::NotFound)?;

    if output.status.success() {
        let content = String::from_utf8(output.stdout)
            .map_err(|_| CredentialError::Parse("Invalid UTF-8 in credentials".to_string()))?
            .trim()
            .to_string();

        parse_credential_json(&content)
    } else {
        // Exit code 44 = item not found, other codes are also treated as not found
        Err(CredentialError::NotFound)
    }
}

/// Get the current system username for Keychain lookup.
fn get_current_username() -> Result<String, CredentialError> {
    std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .map_err(|_| CredentialError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keychain_service_name() {
        assert_eq!(KEYCHAIN_SERVICE, "Claude Code-credentials");
    }

    // Integration test - only runs manually when credentials exist
    #[test]
    #[ignore = "requires real Keychain credentials"]
    fn env_get_token_macos() {
        let result = get_token_macos();
        // If credentials exist, we should get a token
        // If not, we should get NotFound
        match result {
            Ok(token) => {
                assert!(token.starts_with("sk-ant-oat01-"));
                println!("Token retrieved successfully (first 20 chars hidden)");
            }
            Err(CredentialError::NotFound) => {
                println!("No credentials found - expected if not logged in");
            }
            Err(e) => {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}
