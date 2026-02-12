# Version Display

This document records the design decision for version string placement in the
TUI.

## Decision

Version string (`vX.Y.Z`) displayed in the **bottom-right corner of the footer
row**, NOT in the header.

## Implementation

- **Constant**:
  `const VERSION_TEXT: &str = concat!("v", env!("CARGO_PKG_VERSION"))`
- **Rendering**: Right-aligned `Paragraph` overlaying the footer area
- **Header**: Stays as plain "Agent Console Dashboard" (no version)

## Testing

Tests must not hardcode version numbers. Use `env!("CARGO_PKG_VERSION")` for
compile-time comparison.

**Example:**

```rust
let expected_version = format!("v{}", env!("CARGO_PKG_VERSION"));
assert!(footer_text.contains(&expected_version));
```

## History

- **Agent 3 (acd-4mk)**: Initially placed version in the header
- **Fixed in commit f8c2551**: Moved to bottom-right corner per specification
- **Root cause**: Agent drift from issue specification (acd-4mk specified
  bottom-right)
