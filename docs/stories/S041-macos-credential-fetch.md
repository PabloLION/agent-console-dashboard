# Story: macOS Keychain Credential Fetch

**Story ID:** S041 **Epic:**
[E011 - Claude Usage Crate](../epic/E011-claude-usage-crate.md) **Status:**
Draft **Priority:** P0 **Estimated Points:** 3

## Description

As a macOS user, I want the claude-usage crate to fetch OAuth credentials from
the macOS Keychain, So that I can retrieve my API usage without manually
providing credentials.

## Context

Claude Code stores OAuth credentials in the macOS Keychain under the service
name "Claude Code-credentials". The credential is a JSON object containing the
access token needed for API calls. This story implements secure retrieval of
these credentials on macOS.

## Implementation Details

### Technical Approach

1. Use `security-framework` crate for Keychain access
2. Query for generic password with service "Claude Code-credentials"
3. Parse JSON to extract `claudeAiOauth.accessToken`
4. Check `expiresAt` to detect expired tokens
5. Return token or appropriate error
6. Ensure token is not logged or stored beyond immediate use

### Credential Format

```json
{
  "claudeAiOauth": {
    "accessToken": "sk-ant-oat01-...",
    "refreshToken": "sk-ant-ort01-...",
    "expiresAt": 1748658860401,
    "scopes": ["user:inference", "user:profile"]
  }
}
```

### Files to Create/Modify

- `crates/claude-usage/src/credentials/mod.rs` - Credential module
- `crates/claude-usage/src/credentials/macos.rs` - macOS implementation
- `crates/claude-usage/src/error.rs` - Error types

### Dependencies

- [S040 - Workspace Restructure](./S040-workspace-restructure.md)

## Acceptance Criteria

- [ ] Given valid credentials in Keychain, when `get_token()` is called, then
      the access token is returned
- [ ] Given no credentials in Keychain, when `get_token()` is called, then
      `CredentialNotFound` error is returned
- [ ] Given expired credentials, when `get_token()` is called, then
      `TokenExpired` error is returned with guidance to re-login
- [ ] Given malformed credential JSON, when `get_token()` is called, then
      `ParseError` is returned
- [ ] Given successful retrieval, when function returns, then token is not
      retained in memory

## Testing Requirements

- [ ] Unit test: Parse valid credential JSON correctly
- [ ] Unit test: Detect expired token based on expiresAt
- [ ] Unit test: Handle missing accessToken field
- [ ] Integration test: Retrieve real credential from Keychain (manual/CI skip)

## Out of Scope

- Token refresh (Claude Code handles this)
- Credential caching
- Writing credentials to Keychain

## Notes

### Keychain Access

```rust
use security_framework::passwords::get_generic_password;

pub fn get_token_macos() -> Result<String, CredentialError> {
    let password = get_generic_password("Claude Code-credentials", "")
        .map_err(|_| CredentialError::NotFound)?;

    let json: serde_json::Value = serde_json::from_slice(&password)
        .map_err(|e| CredentialError::Parse(e.to_string()))?;

    let oauth = json.get("claudeAiOauth")
        .ok_or(CredentialError::MissingField("claudeAiOauth"))?;

    // Check expiration
    if let Some(expires_at) = oauth.get("expiresAt").and_then(|v| v.as_i64()) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        if now > expires_at {
            return Err(CredentialError::Expired);
        }
    }

    let token = oauth.get("accessToken")
        .and_then(|v| v.as_str())
        .ok_or(CredentialError::MissingField("accessToken"))?;

    Ok(token.to_string())
}
```

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    #[error("Claude Code credentials not found. Run `claude` to login.")]
    NotFound,

    #[error("Credentials expired. Run `claude` to re-login.")]
    Expired,

    #[error("Failed to parse credentials: {0}")]
    Parse(String),

    #[error("Missing field in credentials: {0}")]
    MissingField(&'static str),
}
```

### Security Practices

- Token retrieved → used immediately → dropped
- No logging of token value
- No storage in struct fields beyond function scope
- Use `secrecy` crate if stronger guarantees needed
