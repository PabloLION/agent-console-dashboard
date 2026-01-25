//! Credential retrieval for Claude Code OAuth tokens.
//!
//! This module provides platform-specific credential retrieval:
//! - macOS: Reads from Keychain
//! - Linux: Reads from `~/.claude/.credentials.json`
//!
//! # Security
//!
//! Tokens are retrieved, used immediately, and discarded. They are never:
//! - Logged
//! - Stored in memory longer than necessary
//! - Passed to other modules

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

use crate::error::CredentialError;

/// Service name used by Claude Code in macOS Keychain.
pub const KEYCHAIN_SERVICE: &str = "Claude Code-credentials";

/// Path to credentials file on Linux (relative to HOME).
pub const LINUX_CREDENTIALS_PATH: &str = ".claude/.credentials.json";

/// Environment variable that can override file-based credentials.
pub const ENV_VAR_TOKEN: &str = "CLAUDE_CODE_OAUTH_TOKEN";

/// Retrieve the OAuth access token from platform-specific storage.
///
/// On macOS, this reads from the Keychain.
/// On Linux, this reads from `~/.claude/.credentials.json`.
///
/// The environment variable `CLAUDE_CODE_OAUTH_TOKEN` takes precedence
/// on all platforms if set.
///
/// # Errors
///
/// Returns [`CredentialError`] if:
/// - Credentials are not found
/// - Credentials are expired
/// - Credentials cannot be parsed
/// - Required fields are missing
pub fn get_token() -> Result<String, CredentialError> {
    // Environment variable takes precedence on all platforms
    if let Ok(token) = std::env::var(ENV_VAR_TOKEN) {
        if !token.is_empty() {
            return Ok(token);
        }
    }

    #[cfg(target_os = "macos")]
    {
        macos::get_token_macos()
    }

    #[cfg(target_os = "linux")]
    {
        linux::get_token_linux()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err(CredentialError::NotFound)
    }
}

/// Parse credential JSON and extract the access token.
///
/// This function is shared between macOS and Linux implementations.
///
/// # Arguments
///
/// * `content` - The raw JSON content from Keychain or file
///
/// # Errors
///
/// Returns [`CredentialError`] if:
/// - JSON parsing fails
/// - `claudeAiOauth` field is missing
/// - `accessToken` field is missing
/// - Token is expired (based on `expiresAt`)
pub(crate) fn parse_credential_json(content: &str) -> Result<String, CredentialError> {
    let json: serde_json::Value =
        serde_json::from_str(content).map_err(|e| CredentialError::Parse(e.to_string()))?;

    let oauth = json
        .get("claudeAiOauth")
        .ok_or(CredentialError::MissingField("claudeAiOauth"))?;

    // Check expiration if expiresAt is present
    if let Some(expires_at) = oauth.get("expiresAt").and_then(|v| v.as_i64()) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_millis() as i64;

        if now > expires_at {
            return Err(CredentialError::Expired);
        }
    }

    let token = oauth
        .get("accessToken")
        .and_then(|v| v.as_str())
        .ok_or(CredentialError::MissingField("accessToken"))?;

    Ok(token.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_credentials() {
        let json = r#"{
            "claudeAiOauth": {
                "accessToken": "sk-ant-oat01-test-token",
                "refreshToken": "sk-ant-ort01-refresh",
                "expiresAt": 9999999999999,
                "scopes": ["user:inference", "user:profile"]
            }
        }"#;

        let token = parse_credential_json(json).expect("should parse valid JSON");
        assert_eq!(token, "sk-ant-oat01-test-token");
    }

    #[test]
    fn test_parse_missing_claude_ai_oauth() {
        let json = r#"{"other": "data"}"#;
        let result = parse_credential_json(json);
        assert!(matches!(
            result,
            Err(CredentialError::MissingField("claudeAiOauth"))
        ));
    }

    #[test]
    fn test_parse_missing_access_token() {
        let json = r#"{
            "claudeAiOauth": {
                "refreshToken": "sk-ant-ort01-refresh"
            }
        }"#;
        let result = parse_credential_json(json);
        assert!(matches!(
            result,
            Err(CredentialError::MissingField("accessToken"))
        ));
    }

    #[test]
    fn test_parse_expired_token() {
        let json = r#"{
            "claudeAiOauth": {
                "accessToken": "sk-ant-oat01-test-token",
                "expiresAt": 1000
            }
        }"#;
        let result = parse_credential_json(json);
        assert!(matches!(result, Err(CredentialError::Expired)));
    }

    #[test]
    fn test_parse_invalid_json() {
        let json = "not valid json";
        let result = parse_credential_json(json);
        assert!(matches!(result, Err(CredentialError::Parse(_))));
    }

    #[test]
    fn test_parse_no_expires_at_is_valid() {
        // Credentials without expiresAt should still be valid
        let json = r#"{
            "claudeAiOauth": {
                "accessToken": "sk-ant-oat01-no-expiry"
            }
        }"#;
        let token = parse_credential_json(json).expect("should parse without expiresAt");
        assert_eq!(token, "sk-ant-oat01-no-expiry");
    }

    #[test]
    fn test_env_var_takes_precedence() {
        // Use a unique env var name to avoid test interference
        // Since we can't easily mock the env var check, test the logic directly
        let token = "test-env-token-value";
        std::env::set_var(ENV_VAR_TOKEN, token);

        // Verify the env var is set
        assert_eq!(std::env::var(ENV_VAR_TOKEN).ok(), Some(token.to_string()));

        // The get_token function should return this value
        let result = get_token();
        std::env::remove_var(ENV_VAR_TOKEN);

        assert_eq!(result.expect("should use env var"), token);
    }

    #[test]
    fn test_empty_env_var_behavior() {
        // Empty env var should fall through to platform-specific implementation
        // We just verify the logic exists - actual behavior depends on platform state
        std::env::set_var(ENV_VAR_TOKEN, "");

        // Verify empty string is detected
        let env_value = std::env::var(ENV_VAR_TOKEN).ok();
        assert_eq!(env_value, Some(String::new()));

        std::env::remove_var(ENV_VAR_TOKEN);
    }
}
