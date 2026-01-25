# Story: napi-rs Bindings for npm

**Story ID:** S046 **Epic:**
[E011 - Claude Usage Crate](../epic/E011-claude-usage-crate.md) **Status:**
Implemented **Priority:** P2 **Estimated Points:** 5

## Description

As a Node.js developer, I want to use claude-usage from npm, So that I can fetch
Claude usage data in my JavaScript/TypeScript projects.

## Context

napi-rs allows creating native Node.js addons from Rust code. This provides
better performance than WASM and full access to system APIs (Keychain,
filesystem). The npm package will mirror the Rust API.

## Implementation Details

### Technical Approach

1. Add napi-rs dependencies to claude-usage crate
2. Create napi bindings for `get_usage()` and types
3. Configure napi build for multiple platforms
4. Set up npm package structure
5. Publish to npm as `claude-usage`

### Package Structure

```text
crates/claude-usage/
├── Cargo.toml
├── src/
│   └── lib.rs
├── npm/
│   ├── package.json
│   ├── index.js
│   ├── index.d.ts
│   └── darwin-arm64/
│       └── claude-usage.darwin-arm64.node
```

### Files to Create/Modify

- `crates/claude-usage/Cargo.toml` - Add napi dependencies
- `crates/claude-usage/src/napi.rs` - napi bindings
- `crates/claude-usage/npm/package.json` - npm package manifest
- `crates/claude-usage/npm/index.js` - JS entry point
- `crates/claude-usage/npm/index.d.ts` - TypeScript definitions

### Dependencies

- [S045 - Publish to crates.io](./S045-publish-crates-io.md)

## Acceptance Criteria

- [ ] Given napi build, when compiled on macOS arm64, then native addon is
      produced
- [ ] Given napi build, when compiled on Linux x86_64, then native addon is
      produced
- [ ] Given npm package, when `require('claude-usage')` is called, then module
      loads
- [ ] Given `getUsage()` call in Node.js, when credentials exist, then usage
      data is returned
- [ ] Given TypeScript project, when importing, then types are available

## Testing Requirements

- [ ] Unit test: napi bindings compile
- [ ] Integration test: Node.js can call `getUsage()`
- [ ] Integration test: TypeScript types match runtime behavior

## Out of Scope

- Browser support (napi-rs is Node.js only)
- Deno support
- Bun support (may work, not tested)

## Notes

### Cargo.toml Additions

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
napi = { version = "2", features = ["async"] }
napi-derive = "2"

[build-dependencies]
napi-build = "2"

[features]
default = []
napi = ["dep:napi", "dep:napi-derive"]
```

### napi Bindings

```rust
// src/napi.rs
use napi_derive::napi;

#[napi(object)]
pub struct JsUsagePeriod {
    pub utilization: f64,
    pub resets_at: String,
}

#[napi(object)]
pub struct JsUsageData {
    pub five_hour: JsUsagePeriod,
    pub seven_day: JsUsagePeriod,
}

#[napi]
pub fn get_usage() -> napi::Result<JsUsageData> {
    let usage = crate::get_usage()
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;

    Ok(JsUsageData {
        five_hour: JsUsagePeriod {
            utilization: usage.five_hour.utilization,
            resets_at: usage.five_hour.resets_at.to_rfc3339(),
        },
        seven_day: JsUsagePeriod {
            utilization: usage.seven_day.utilization,
            resets_at: usage.seven_day.resets_at.to_rfc3339(),
        },
    })
}

#[napi]
pub async fn get_usage_async() -> napi::Result<JsUsageData> {
    // Async version
}
```

### package.json

```json
{
  "name": "claude-usage",
  "version": "0.1.0",
  "description": "Fetch Claude API usage data from Anthropic",
  "main": "index.js",
  "types": "index.d.ts",
  "napi": {
    "name": "claude-usage",
    "triples": {
      "defaults": true,
      "additional": ["aarch64-apple-darwin"]
    }
  },
  "scripts": {
    "build": "napi build --platform --release",
    "prepublishOnly": "napi prepublish -t npm"
  },
  "repository": {
    "type": "git",
    "url": "https://github.com/PabloLION/agent-console-dashboard"
  },
  "keywords": ["claude", "anthropic", "usage", "api"],
  "license": "MIT OR Apache-2.0"
}
```

### TypeScript Definitions

```typescript
// index.d.ts
export interface UsagePeriod {
  utilization: number;
  resetsAt: string;
}

export interface UsageData {
  fiveHour: UsagePeriod;
  sevenDay: UsagePeriod;
}

export function getUsage(): UsageData;
export function getUsageAsync(): Promise<UsageData>;
```

### Build Matrix

| Platform | Architecture | Target                    |
| -------- | ------------ | ------------------------- |
| macOS    | arm64        | aarch64-apple-darwin      |
| macOS    | x64          | x86_64-apple-darwin       |
| Linux    | x64          | x86_64-unknown-linux-gnu  |
| Linux    | arm64        | aarch64-unknown-linux-gnu |

### Publishing

```bash
cd crates/claude-usage
npm publish
```
