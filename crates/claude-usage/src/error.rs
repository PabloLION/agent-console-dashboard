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

/// Errors that can occur when calling the Anthropic API.
#[derive(Debug, Error)]
pub enum ApiError {
    /// Network error during HTTP request.
    #[error("Network error: {0}")]
    Network(String),

    /// API returned 401 Unauthorized - token is invalid or expired.
    #[error("Unauthorized. Run `claude` to re-login.")]
    Unauthorized,

    /// API returned 429 Too Many Requests.
    #[error("Rate limited. Retry after: {retry_after:?}")]
    RateLimited {
        /// Value of the retry-after header, if present.
        retry_after: Option<String>,
    },

    /// API returned 5xx server error.
    #[error("Server error: {0}")]
    Server(u16),

    /// API returned an unexpected status code.
    #[error("Unexpected status code: {0}")]
    Unexpected(u16),
}
