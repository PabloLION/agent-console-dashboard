# Version Display

This document records the design decision for version string placement in the
TUI.

## Decision

Version string (`vX.Y.Z`) displayed **right-aligned in the header row**, NOT in
the footer.

The footer bottom-right is reserved for API usage display (acd-0i4i, future
work).

## Implementation

- **Constant**:
  `const VERSION_TEXT: &str = concat!("v", env!("CARGO_PKG_VERSION"))`
- **Rendering**: Right-aligned `Paragraph` overlaying the header area
- **Footer**: Reserved for API usage (not yet implemented)

## Testing

Tests must not hardcode version numbers. Use `env!("CARGO_PKG_VERSION")` for
compile-time comparison.

**Example:**

```rust
let expected_version = format!("v{}", env!("CARGO_PKG_VERSION"));
assert!(header_text.contains(&expected_version));
```

## History

- **Agent 3 (acd-4mk)**: Initially placed version in the header
- **Commit f8c2551**: Moved to bottom-right footer per original spec (acd-4mk)
- **acd-mq6y**: Moved back to header right-aligned; footer bottom-right reserved
  for API usage (acd-0i4i)
