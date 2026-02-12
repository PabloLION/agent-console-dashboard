# Testing Strategy

**Date**: 2026-01-24 **Status**: Accepted

## Context

During QC review of Epic 1, we discovered that tests in
`tests/client_auto_start.rs` were trying to access the internal `client` module,
which is not part of the public API. This raised questions about how to properly
organize tests in Rust.

## Decision

We adopt the standard Rust testing convention:

### Test Organization

| Test Type             | Location                              | Access Level           | Purpose                      |
| --------------------- | ------------------------------------- | ---------------------- | ---------------------------- |
| **Unit tests**        | Inside `src/*.rs` with `#[cfg(test)]` | Private + public items | Test internal implementation |
| **Integration tests** | `tests/` folder                       | Public API only        | Test as external consumer    |
| **E2E tests**         | TBD (future)                          | Full system            | Test complete workflows      |

### Key Rules

1. **Unit tests go inside source files** using
   `#[cfg(test)] mod tests { use super::*; }`
2. **Integration tests can only access public API** (items exported from
   `lib.rs`)
3. **Internal modules** (`pub(crate)`) should have unit tests, not integration
   tests
4. **The `tests/` folder** is for testing the public API as an external user
   would

### The Distinction

The difference between unit and integration tests is NOT about system
dependencies. It's about **perspective and access level**:

- **Unit tests**: Test as the code author (can access private items)
- **Integration tests**: Test as a library consumer (public API only)

## Consequences

### Positive

- Clear separation of concerns
- Tests live close to the code they test
- Internal refactoring doesn't break integration tests
- Follows Rust community conventions

### Implementation

- Moved `tests/client_auto_start.rs` → `src/client/connection.rs` as unit tests
- Client module is `pub(crate)` (internal only)
- `tests/` folder reserved for future public API tests

## Test Grouping

**Date**: 2026-02-08

Tests that require external resources (network, env vars, running services)
should be excluded from the default `cargo test` run.

### Mechanism: `#[ignore]`

Use `#[ignore = "reason"]` to mark tests that need external resources.

```rust
#[test]
#[ignore = "requires network"]
fn net_fetch_usage_data() { /* ... */ }

#[test]
#[ignore = "requires credentials"]
fn env_get_token_macos() { /* ... */ }
```

### Filtering: name/module only

The `#[ignore]` reason string is **documentation only** — it is not filterable.
Rust's test harness provides no way to select ignored tests by reason.

Filtering options:

| Method           | Command                            | Filters by            |
| ---------------- | ---------------------------------- | --------------------- |
| Run all ignored  | `cargo test -- --ignored`          | All `#[ignore]` tests |
| Filter by prefix | `cargo test -- --ignored env_`    | Test name substring   |
| Filter by module | `cargo test client:: -- --ignored` | Module path           |

The reason string appears in `cargo test` output as
`test name ... ignored, reason` but cannot be used as a filter argument.

### Naming convention

Prefix ignored test names with a category, separated by single underscore
(`_`). The prefix clearly indicates the test's external dependency type.

| Prefix  | Meaning                         | Example                 |
| ------- | ------------------------------- | ----------------------- |
| `net_` | Requires network                | `net_fetch_usage_data` |
| `env_` | Requires env vars / credentials | `env_get_token_macos`  |
| `svc_` | Requires running service        | `svc_daemon_responds`  |

Run by category: `cargo test -- --ignored env_`

### Alternatives considered

| Approach                                  | Pros                                   | Cons                                                    |
| ----------------------------------------- | -------------------------------------- | ------------------------------------------------------- |
| `#[ignore]` + naming                      | Simple, standard Rust, no build config | Manual prefix discipline                                |
| Feature flags (`#[cfg(feature = "...")]`) | Code excluded from binary              | Heavyweight, tests invisible by default, easy to forget |
| cargo-nextest                             | Rich filtering syntax                  | External dependency, CI overhead                        |

### Hook integration

- **Pre-commit**: `cargo test` (fast, no ignored tests)
- **Pre-push**: `cargo test -- --ignored` (all tests including network/env)

The pre-push hook should only run ignored tests after the failing
`test_fetch_usage_raw_integration` is fixed (it panics when
`CLAUDE_CODE_OAUTH_TOKEN` is missing instead of skipping gracefully).

## References

- [The Rust Book - Test Organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html)
- Lesson saved:
  `~/.claude/pablo/lessons/20260124-091500-rust-testing-internal-modules.md`
