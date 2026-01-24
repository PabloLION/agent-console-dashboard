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

## References

- [The Rust Book - Test Organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html)
- Lesson saved:
  `~/.claude/pablo/lessons/20260124-091500-rust-testing-internal-modules.md`
