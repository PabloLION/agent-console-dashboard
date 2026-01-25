//! Error types for the claude-usage crate.

use thiserror::Error;

/// Errors that can occur when retrieving credentials.
#[derive(Debug, Error)]
pub enum CredentialError {
    /// Claude Code credentials not found in the platform's secure storage.
    #[error("Claude Code credentials not found. Run `claude` to login.")]
    NotFound,

    /// Credentials have expired and need to be refreshed.
    #[error("Credentials expired. Run `claude` to re-login.")]
    Expired,

    /// Failed to parse the credential JSON.
    #[error("Failed to parse credentials: {0}")]
    Parse(String),

    /// Required field is missing from credentials.
    #[error("Missing field in credentials: {0}")]
    MissingField(&'static str),

    /// Permission denied when accessing credentials.
    #[error("Permission denied accessing credentials: {0}")]
    Permission(String),

    /// I/O error when reading credentials.
    #[error("I/O error reading credentials: {0}")]
    Io(String),

    /// HOME directory not set (Linux/Unix).
    #[error("HOME environment variable not set")]
    NoHomeDir,
}
