//! macOS Keychain credential retrieval.
//!
//! This module retrieves Claude Code OAuth credentials from the macOS Keychain.
//! The credentials are stored by Claude Code under the service name
//! "Claude Code-credentials".

use security_framework::passwords::get_generic_password;

use super::{parse_credential_json, KEYCHAIN_SERVICE};
use crate::error::CredentialError;

/// Retrieve the OAuth access token from macOS Keychain.
///
/// # Errors
///
/// Returns [`CredentialError`] if:
/// - Credentials are not found in Keychain
/// - Credentials cannot be parsed
/// - Token is expired
pub fn get_token_macos() -> Result<String, CredentialError> {
    // Query Keychain for the Claude Code credentials
    // Account name is empty string as used by Claude Code
    let password =
        get_generic_password(KEYCHAIN_SERVICE, "").map_err(|_| CredentialError::NotFound)?;

    // Convert bytes to string - use generic message to avoid exposing credential bytes
    let content = String::from_utf8(password)
        .map_err(|_| CredentialError::Parse("Invalid UTF-8 in credentials".to_string()))?;

    // Parse and extract token
    parse_credential_json(&content)
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
    fn test_get_token_macos_integration() {
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
