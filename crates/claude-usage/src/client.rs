//! HTTP client for the Anthropic usage API.
//!
//! This module provides functions to fetch usage data from the Anthropic API.
//! It handles authentication, headers, and error mapping.

use crate::error::ApiError;

/// Anthropic OAuth usage API endpoint.
pub const USAGE_API_URL: &str = "https://api.anthropic.com/api/oauth/usage";

/// Required beta header value for OAuth endpoints.
pub const BETA_HEADER: &str = "oauth-2025-04-20";

/// Fetch raw usage data from the Anthropic API (blocking).
///
/// This function makes a synchronous HTTP request to the usage API
/// and returns the raw JSON response body.
///
/// # Arguments
///
/// * `token` - OAuth access token for authentication
///
/// # Errors
///
/// Returns [`ApiError`] if:
/// - Network request fails
/// - Server returns 401 (unauthorized)
/// - Server returns 429 (rate limited)
/// - Server returns 5xx (server error)
/// - Server returns unexpected status code
///
/// # Security
///
/// The token is used only for this request and is not stored.
#[cfg(feature = "blocking")]
pub fn fetch_usage_raw(token: &str) -> Result<String, ApiError> {
    let client = reqwest::blocking::Client::new();

    let response = client
        .get(USAGE_API_URL)
        .header("Authorization", format!("Bearer {}", token))
        .header("anthropic-beta", BETA_HEADER)
        .send()
        .map_err(|e| ApiError::Network(e.to_string()))?;

    map_response(response)
}

/// Map HTTP response to result, handling error status codes.
#[cfg(feature = "blocking")]
fn map_response(response: reqwest::blocking::Response) -> Result<String, ApiError> {
    let status = response.status().as_u16();

    match status {
        200 => response
            .text()
            .map_err(|e| ApiError::Network(e.to_string())),
        401 => Err(ApiError::Unauthorized),
        429 => {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(String::from);
            Err(ApiError::RateLimited { retry_after })
        }
        500..=599 => Err(ApiError::Server(status)),
        _ => Err(ApiError::Unexpected(status)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_url_is_correct() {
        assert_eq!(USAGE_API_URL, "https://api.anthropic.com/api/oauth/usage");
    }

    #[test]
    fn test_beta_header_is_correct() {
        assert_eq!(BETA_HEADER, "oauth-2025-04-20");
    }

    // Integration test - requires valid token
    #[test]
    #[ignore = "requires real API credentials"]
    #[cfg(feature = "blocking")]
    fn test_fetch_usage_raw_integration() {
        // This test requires CLAUDE_CODE_OAUTH_TOKEN env var or real credentials
        let token = std::env::var("CLAUDE_CODE_OAUTH_TOKEN")
            .expect("CLAUDE_CODE_OAUTH_TOKEN must be set for integration test");

        let result = fetch_usage_raw(&token);
        match result {
            Ok(body) => {
                assert!(body.contains("five_hour"));
                assert!(body.contains("seven_day"));
                println!("API response received successfully");
            }
            Err(ApiError::Unauthorized) => {
                println!("Token is invalid or expired");
            }
            Err(e) => {
                panic!("Unexpected error: {}", e);
            }
        }
    }

    #[test]
    #[cfg(feature = "blocking")]
    fn test_fetch_with_invalid_token() {
        // Test that invalid token returns Unauthorized
        let result = fetch_usage_raw("invalid-token");
        assert!(matches!(result, Err(ApiError::Unauthorized)));
    }
}
