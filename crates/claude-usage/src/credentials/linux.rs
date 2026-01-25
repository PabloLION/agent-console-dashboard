//! Linux credential file retrieval.
//!
//! This module retrieves Claude Code OAuth credentials from the credential file
//! at `~/.claude/.credentials.json`. This is the standard location used by
//! Claude Code on Linux systems.

use std::fs;
use std::path::PathBuf;

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
    let content = fs::read_to_string(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => CredentialError::NotFound,
        std::io::ErrorKind::PermissionDenied => {
            CredentialError::Permission(path.display().to_string())
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
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

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

    #[test]
    fn test_credentials_path_format() {
        assert_eq!(LINUX_CREDENTIALS_PATH, ".claude/.credentials.json");
    }

    #[test]
    fn test_get_credentials_path_uses_home() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let _guard = HomeGuard::set(temp_dir.path());

        let path = get_credentials_path().expect("should get path");
        let expected = temp_dir.path().join(".claude/.credentials.json");
        assert_eq!(path, expected);
    }

    #[test]
    fn test_get_credentials_path_no_home() {
        // Use a special guard that removes HOME instead of setting it
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
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("create .claude dir");

        let creds_path = claude_dir.join(".credentials.json");
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

        let _guard = HomeGuard::set(temp_dir.path());
        let result = get_token_linux();

        assert_eq!(
            result.expect("should read token"),
            "sk-ant-oat01-linux-test-token"
        );
    }

    #[test]
    fn test_missing_credentials_file() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let _guard = HomeGuard::set(temp_dir.path());

        let result = get_token_linux();
        assert!(matches!(result, Err(CredentialError::NotFound)));
    }

    #[test]
    fn test_invalid_json_in_file() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("create .claude dir");

        let creds_path = claude_dir.join(".credentials.json");
        let mut file = File::create(&creds_path).expect("create credentials file");
        writeln!(file, "not valid json").expect("write invalid content");

        let _guard = HomeGuard::set(temp_dir.path());
        let result = get_token_linux();

        assert!(matches!(result, Err(CredentialError::Parse(_))));
    }

    #[test]
    fn test_expired_token_in_file() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("create .claude dir");

        let creds_path = claude_dir.join(".credentials.json");
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

        let _guard = HomeGuard::set(temp_dir.path());
        let result = get_token_linux();

        assert!(matches!(result, Err(CredentialError::Expired)));
    }

    // Integration test - only runs manually
    #[test]
    #[ignore = "requires real credentials file"]
    fn test_get_token_linux_integration() {
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
