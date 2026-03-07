//! HTTP client for the Anthropic usage API.
//!
//! This module provides functions to fetch usage data from the Anthropic API.
//! It handles authentication, headers, and error mapping.

use crate::error::ApiError;
use crate::types::{UsageData, UsagePeriod};

/// Anthropic OAuth usage API endpoint.
pub const USAGE_API_URL: &str = "https://api.anthropic.com/api/oauth/usage";

/// Anthropic messages API endpoint (used for header-based fallback).
pub const MESSAGES_API_URL: &str = "https://api.anthropic.com/v1/messages";

/// Required beta header value for OAuth endpoints.
pub const BETA_HEADER: &str = "oauth-2025-04-20";

/// Model used for the 1-token probe request in the header fallback.
///
/// Uses the cheapest available model to minimize cost (~1 token per request).
pub const PROBE_MODEL: &str = "claude-haiku-4-5";

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
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|_| ApiError::Network("Failed to build HTTP client".to_string()))?;

    let response = client
        .get(USAGE_API_URL)
        .header("Authorization", format!("Bearer {}", token))
        .header("anthropic-beta", BETA_HEADER)
        .send()
        // Use generic message to avoid any potential token exposure in error details
        .map_err(|_| ApiError::Network("Failed to connect to Anthropic API".to_string()))?;

    map_response(response)
}

/// Map HTTP response to result, handling error status codes.
#[cfg(feature = "blocking")]
fn map_response(response: reqwest::blocking::Response) -> Result<String, ApiError> {
    let status = response.status().as_u16();

    match status {
        200 => response
            .text()
            .map_err(|_| ApiError::Network("Failed to read response body".to_string())),
        401 => Err(ApiError::Unauthorized),
        403 => Err(ApiError::Forbidden),
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

/// Fetch usage data via a 1-token inference probe and parse rate limit headers.
///
/// This is the fallback when `/api/oauth/usage` returns 403 (scope mismatch).
/// Sends `POST /v1/messages` with `max_tokens=1` and reads
/// `anthropic-ratelimit-unified-*` response headers, which expose the same
/// utilization data accessible with `user:inference` scope only.
///
/// Header mappings:
/// - `anthropic-ratelimit-unified-5h-utilization` → `five_hour.utilization`
/// - `anthropic-ratelimit-unified-5h-reset` → `five_hour.resets_at` (Unix → DateTime)
/// - `anthropic-ratelimit-unified-7d-utilization` → `seven_day.utilization`
/// - `anthropic-ratelimit-unified-7d-reset` → `seven_day.resets_at` (Unix → DateTime)
///
/// # Errors
///
/// Returns [`ApiError`] if:
/// - Network request fails
/// - Server returns 401 (unauthorized)
/// - Server returns 429 (rate limited)
/// - Server returns 5xx (server error)
/// - Server returns unexpected status code
/// - Required headers are missing (returns [`ApiError::MissingHeaders`])
#[cfg(feature = "blocking")]
pub fn fetch_usage_from_headers(token: &str) -> Result<UsageData, ApiError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|_| ApiError::Network("Failed to build HTTP client".to_string()))?;

    let body = r#"{"model":"claude-haiku-4-5","max_tokens":1,"messages":[{"role":"user","content":"quota"}]}"#;

    let response = client
        .post(MESSAGES_API_URL)
        .header("x-api-key", token)
        .header("Content-Type", "application/json")
        .header("anthropic-version", "2023-06-01")
        .body(body)
        .send()
        .map_err(|_| ApiError::Network("Failed to connect to Anthropic API".to_string()))?;

    let status = response.status().as_u16();
    match status {
        401 => return Err(ApiError::Unauthorized),
        403 => return Err(ApiError::Forbidden),
        429 => {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(String::from);
            return Err(ApiError::RateLimited { retry_after });
        }
        500..=599 => return Err(ApiError::Server(status)),
        200 | 201 => {}
        _ => return Err(ApiError::Unexpected(status)),
    }

    parse_usage_from_headers(response.headers())
}

/// Parse [`UsageData`] from Anthropic rate limit response headers.
///
/// Reads `anthropic-ratelimit-unified-5h-utilization`, `5h-reset`,
/// `7d-utilization`, and `7d-reset` headers. Returns
/// [`ApiError::MissingHeaders`] if any required header is absent or
/// unparseable.
#[cfg(feature = "blocking")]
fn parse_usage_from_headers(headers: &reqwest::header::HeaderMap) -> Result<UsageData, ApiError> {
    use chrono::{DateTime, TimeZone, Utc};

    let get_f64 = |name: &str| -> Option<f64> {
        headers
            .get(name)
            .and_then(|v: &reqwest::header::HeaderValue| v.to_str().ok())
            .and_then(|s| s.parse::<f64>().ok())
    };

    let get_unix_dt = |name: &str| -> Option<DateTime<Utc>> {
        headers
            .get(name)
            .and_then(|v: &reqwest::header::HeaderValue| v.to_str().ok())
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|ts| Utc.timestamp_opt(ts, 0).single())
    };

    let five_hour_utilization = get_f64("anthropic-ratelimit-unified-5h-utilization");
    let seven_day_utilization = get_f64("anthropic-ratelimit-unified-7d-utilization");

    match (five_hour_utilization, seven_day_utilization) {
        (Some(fh_util), Some(sd_util)) => Ok(UsageData {
            five_hour: UsagePeriod {
                utilization: fh_util,
                resets_at: get_unix_dt("anthropic-ratelimit-unified-5h-reset"),
            },
            seven_day: UsagePeriod {
                utilization: sd_util,
                resets_at: get_unix_dt("anthropic-ratelimit-unified-7d-reset"),
            },
            seven_day_sonnet: None,
            extra_usage: None,
        }),
        _ => Err(ApiError::MissingHeaders),
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

    #[test]
    fn test_forbidden_error_display() {
        let err = ApiError::Forbidden;
        assert_eq!(
            err.to_string(),
            "Forbidden: OAuth token not authorized for third-party usage"
        );
    }

    #[test]
    fn test_forbidden_error_is_distinct_from_unauthorized() {
        let forbidden = ApiError::Forbidden;
        let unauthorized = ApiError::Unauthorized;
        // They should have different display strings
        assert_ne!(forbidden.to_string(), unauthorized.to_string());
    }

    // Integration test - requires valid token
    #[test]
    #[ignore = "requires real API credentials"]
    #[cfg(feature = "blocking")]
    fn env_fetch_usage_raw() {
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
    #[ignore = "requires network access to Anthropic API"]
    #[cfg(feature = "blocking")]
    fn test_fetch_with_invalid_token() {
        // Test that invalid token returns Unauthorized
        let result = fetch_usage_raw("invalid-token");
        assert!(matches!(result, Err(ApiError::Unauthorized)));
    }

    #[test]
    #[cfg(feature = "blocking")]
    fn test_parse_usage_from_headers_full() {
        use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
        use std::str::FromStr;

        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_str("anthropic-ratelimit-unified-5h-utilization").expect("valid name"),
            HeaderValue::from_static("72.0"),
        );
        headers.insert(
            HeaderName::from_str("anthropic-ratelimit-unified-5h-reset").expect("valid name"),
            HeaderValue::from_static("1772096400"),
        );
        headers.insert(
            HeaderName::from_str("anthropic-ratelimit-unified-7d-utilization").expect("valid name"),
            HeaderValue::from_static("34.0"),
        );
        headers.insert(
            HeaderName::from_str("anthropic-ratelimit-unified-7d-reset").expect("valid name"),
            HeaderValue::from_static("1772463600"),
        );

        let result = parse_usage_from_headers(&headers);
        let usage = result.expect("should parse successfully");

        assert!((usage.five_hour.utilization - 72.0).abs() < f64::EPSILON);
        assert!((usage.seven_day.utilization - 34.0).abs() < f64::EPSILON);
        assert!(usage.five_hour.resets_at.is_some());
        assert!(usage.seven_day.resets_at.is_some());

        // Verify Unix timestamp conversion
        let fh_reset = usage.five_hour.resets_at.expect("5h reset present");
        assert_eq!(fh_reset.timestamp(), 1_772_096_400);
        let sd_reset = usage.seven_day.resets_at.expect("7d reset present");
        assert_eq!(sd_reset.timestamp(), 1_772_463_600);

        // seven_day_sonnet and extra_usage should be None (not in headers)
        assert!(usage.seven_day_sonnet.is_none());
        assert!(usage.extra_usage.is_none());
    }

    #[test]
    #[cfg(feature = "blocking")]
    fn test_parse_usage_from_headers_missing_utilization() {
        use reqwest::header::HeaderMap;

        // Empty headers — no utilization values
        let headers = HeaderMap::new();
        let result = parse_usage_from_headers(&headers);
        assert!(matches!(result, Err(ApiError::MissingHeaders)));
    }

    #[test]
    #[cfg(feature = "blocking")]
    fn test_parse_usage_from_headers_partial_utilization() {
        use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
        use std::str::FromStr;

        // Only 5h utilization present, 7d missing — should fail
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_str("anthropic-ratelimit-unified-5h-utilization").expect("valid name"),
            HeaderValue::from_static("50.0"),
        );

        let result = parse_usage_from_headers(&headers);
        assert!(matches!(result, Err(ApiError::MissingHeaders)));
    }

    #[test]
    #[cfg(feature = "blocking")]
    fn test_parse_usage_from_headers_missing_reset_times() {
        use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
        use std::str::FromStr;

        // Utilization present but no reset timestamps — should succeed with None resets
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_str("anthropic-ratelimit-unified-5h-utilization").expect("valid name"),
            HeaderValue::from_static("15.5"),
        );
        headers.insert(
            HeaderName::from_str("anthropic-ratelimit-unified-7d-utilization").expect("valid name"),
            HeaderValue::from_static("88.3"),
        );

        let result = parse_usage_from_headers(&headers);
        let usage = result.expect("should parse with missing reset times");

        assert!((usage.five_hour.utilization - 15.5).abs() < f64::EPSILON);
        assert!((usage.seven_day.utilization - 88.3).abs() < f64::EPSILON);
        assert!(usage.five_hour.resets_at.is_none());
        assert!(usage.seven_day.resets_at.is_none());
    }

    #[test]
    #[cfg(feature = "blocking")]
    fn test_parse_usage_from_headers_invalid_utilization_value() {
        use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
        use std::str::FromStr;

        // Malformed utilization value — should fail
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_str("anthropic-ratelimit-unified-5h-utilization").expect("valid name"),
            HeaderValue::from_static("not-a-number"),
        );
        headers.insert(
            HeaderName::from_str("anthropic-ratelimit-unified-7d-utilization").expect("valid name"),
            HeaderValue::from_static("34.0"),
        );

        let result = parse_usage_from_headers(&headers);
        assert!(matches!(result, Err(ApiError::MissingHeaders)));
    }

    #[test]
    #[cfg(feature = "blocking")]
    fn test_messages_api_url_is_correct() {
        assert_eq!(MESSAGES_API_URL, "https://api.anthropic.com/v1/messages");
    }

    #[test]
    #[cfg(feature = "blocking")]
    fn test_probe_model_is_set() {
        assert_eq!(PROBE_MODEL, "claude-haiku-4-5");
    }

    #[test]
    #[cfg(feature = "blocking")]
    fn test_missing_headers_error_display() {
        let err = ApiError::MissingHeaders;
        assert_eq!(
            err.to_string(),
            "Required rate limit headers missing from API response"
        );
    }

    // Integration test - requires valid token with user:inference scope only
    #[test]
    #[ignore = "requires real API credentials"]
    #[cfg(feature = "blocking")]
    fn env_fetch_usage_from_headers() {
        let token = std::env::var("CLAUDE_CODE_OAUTH_TOKEN")
            .expect("CLAUDE_CODE_OAUTH_TOKEN must be set for integration test");

        let result = fetch_usage_from_headers(&token);
        match result {
            Ok(usage) => {
                assert!(usage.five_hour.utilization >= 0.0);
                assert!(usage.seven_day.utilization >= 0.0);
                println!("5h utilization: {}%", usage.five_hour.utilization);
                println!("7d utilization: {}%", usage.seven_day.utilization);
            }
            Err(ApiError::Unauthorized) => {
                println!("Token is invalid or expired");
            }
            Err(ApiError::MissingHeaders) => {
                println!("Rate limit headers not present in response");
            }
            Err(e) => {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}
