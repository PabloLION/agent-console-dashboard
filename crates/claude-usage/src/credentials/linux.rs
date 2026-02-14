//! Linux credential file retrieval.
//!
//! This module retrieves Claude Code OAuth credentials from the credential file
//! at `~/.claude/.credentials.json`. This is the standard location used by
//! Claude Code on Linux systems.

use std::fs;
use std::path::{Path, PathBuf};

use super::{parse_credential_json, LINUX_CREDENTIALS_PATH};
use crate::error::CredentialError;

/// Retrieve the OAuth access token from the Linux credential file.
///
/// # Errors
///
/// Returns [`CredentialError`] if:
/// - HOME environment variable is not set
/// - Credentials file does not exist
/// - File permissions prevent reading
/// - Credentials cannot be parsed
/// - Token is expired
pub fn get_token_linux() -> Result<String, CredentialError> {
    let path = get_credentials_path()?;
    get_token_from_path(&path)
}

/// Retrieve the OAuth access token from a specific credential file path.
///
/// This function is the testable core of credential retrieval, separated from
/// path resolution to avoid environment variable mutation in tests.
///
/// # Errors
///
/// Returns [`CredentialError`] if:
/// - Credentials file does not exist
/// - File permissions prevent reading
/// - Credentials cannot be parsed
/// - Token is expired
fn get_token_from_path(creds_path: &Path) -> Result<String, CredentialError> {
    let content = fs::read_to_string(creds_path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => CredentialError::NotFound,
        std::io::ErrorKind::PermissionDenied => {
            CredentialError::Permission(creds_path.display().to_string())
        }
        _ => CredentialError::Io(e.to_string()),
    })?;

    parse_credential_json(&content)
}

/// Get the path to the credentials file.
fn get_credentials_path() -> Result<PathBuf, CredentialError> {
    let home = std::env::var("HOME").map_err(|_| CredentialError::NoHomeDir)?;
    Ok(PathBuf::from(home).join(LINUX_CREDENTIALS_PATH))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_credentials_path_format() {
        assert_eq!(LINUX_CREDENTIALS_PATH, ".claude/.credentials.json");
    }

    #[test]
    #[serial]
    fn test_get_credentials_path_uses_home() {
        /// RAII guard for HOME environment variable - ensures cleanup even on panic.
        struct HomeGuard(Option<String>);

        impl HomeGuard {
            fn set(path: &std::path::Path) -> Self {
                let original = std::env::var("HOME").ok();
                std::env::set_var("HOME", path);
                Self(original)
            }
        }

        impl Drop for HomeGuard {
            fn drop(&mut self) {
                match &self.0 {
                    Some(home) => std::env::set_var("HOME", home),
                    None => std::env::remove_var("HOME"),
                }
            }
        }

        let temp_dir = TempDir::new().expect("create temp dir");
        let _guard = HomeGuard::set(temp_dir.path());

        let path = get_credentials_path().expect("should get path");
        let expected = temp_dir.path().join(".claude/.credentials.json");
        assert_eq!(path, expected);
    }

    #[test]
    #[serial]
    fn test_get_credentials_path_no_home() {
        /// RAII guard that removes HOME environment variable.
        struct NoHomeGuard(Option<String>);

        impl NoHomeGuard {
            fn new() -> Self {
                let original = std::env::var("HOME").ok();
                std::env::remove_var("HOME");
                Self(original)
            }
        }

        impl Drop for NoHomeGuard {
            fn drop(&mut self) {
                if let Some(home) = &self.0 {
                    std::env::set_var("HOME", home);
                }
            }
        }

        let _guard = NoHomeGuard::new();
        let result = get_credentials_path();
        assert!(matches!(result, Err(CredentialError::NoHomeDir)));
    }

    #[test]
    fn test_read_valid_credentials_file() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let creds_path = temp_dir.path().join(".credentials.json");
        let mut file = File::create(&creds_path).expect("create credentials file");
        writeln!(
            file,
            r#"{{
            "claudeAiOauth": {{
                "accessToken": "sk-ant-oat01-linux-test-token",
                "expiresAt": 9999999999999
            }}
        }}"#
        )
        .expect("write credentials");

        let result = get_token_from_path(&creds_path);

        assert_eq!(
            result.expect("should read token"),
            "sk-ant-oat01-linux-test-token"
        );
    }

    #[test]
    fn test_missing_credentials_file() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let creds_path = temp_dir.path().join(".credentials.json");

        let result = get_token_from_path(&creds_path);
        assert!(matches!(result, Err(CredentialError::NotFound)));
    }

    #[test]
    fn test_invalid_json_in_file() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let creds_path = temp_dir.path().join(".credentials.json");
        let mut file = File::create(&creds_path).expect("create credentials file");
        writeln!(file, "not valid json").expect("write invalid content");

        let result = get_token_from_path(&creds_path);

        assert!(matches!(result, Err(CredentialError::Parse(_))));
    }

    #[test]
    fn test_expired_token_in_file() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let creds_path = temp_dir.path().join(".credentials.json");
        let mut file = File::create(&creds_path).expect("create credentials file");
        writeln!(
            file,
            r#"{{
            "claudeAiOauth": {{
                "accessToken": "sk-ant-oat01-expired-token",
                "expiresAt": 1000
            }}
        }}"#
        )
        .expect("write expired credentials");

        let result = get_token_from_path(&creds_path);

        assert!(matches!(result, Err(CredentialError::Expired)));
    }

    // Integration test - only runs manually
    #[test]
    #[ignore = "requires real credentials file"]
    fn env_get_token_linux() {
        let result = get_token_linux();
        match result {
            Ok(token) => {
                assert!(token.starts_with("sk-ant-oat01-"));
                println!("Token retrieved successfully");
            }
            Err(CredentialError::NotFound) => {
                println!("No credentials file found - expected if not logged in");
            }
            Err(e) => {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}
