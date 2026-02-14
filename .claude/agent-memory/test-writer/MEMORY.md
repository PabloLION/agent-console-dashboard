# Test Writer Agent Memory

## Testing Patterns

### TestBackend Rendering Tests

TUI rendering tests use `ratatui::backend::TestBackend` for verifying visual
output without a real terminal. Key patterns:

- Helper: `render_session_list_to_buffer()` in `src/tui/test_utils.rs` renders
  session list to buffer
- Buffer inspection: `find_row_with_text()`, `row_text()` to extract rendered
  content
- Always check both the logic (unit tests) AND the rendering (TestBackend tests)

Example structure:

```rust
let sessions = vec![make_test_session_with_dir(...)];
let buffer = render_session_list_to_buffer(&sessions, Some(0), 100, 10);
let row = find_row_with_text(&buffer, "search-text").expect("should find row");
let row_text = row_text(&buffer, row);
assert!(row_text.contains("expected-output"));
```

### Directory Disambiguation Tests (acd-0ci)

Location: `crates/agent-console-dashboard/src/tui/views/dashboard/tests/`

- `disambiguation.rs`: unit tests for `compute_directory_display_names()` logic
- `rendering.rs`: TestBackend tests verifying TUI displays disambiguated names

Pattern: two sessions with same basename (e.g., `/foo/project` and
`/bar/project`) should display as `foo/project` and `bar/project` in the TUI.

### Test File Organization

- Unit tests: `src/*/tests/` directories with `mod.rs` + submodules
- Integration tests: `tests/` directory at crate root
- Test utilities: `src/tui/test_utils.rs` for TUI helpers, `src/test_utils.rs`
  for general helpers
- Use `pub(crate) use super::*;` in `tests/mod.rs` for visibility

### Version References in Tests

Never hardcode version numbers like "0.1.2" in tests. Use
`env!("CARGO_PKG_VERSION")` to access the version from Cargo.toml dynamically.

### Eliminating Environment Variable Mutation in Tests

Pattern for refactoring tests that mutate env vars (example from
`crates/claude-usage/src/credentials/linux.rs`):

1. **Separate concerns**: Extract path-based logic into a new function that
   takes `&Path` instead of reading from environment
2. **Keep original function**: Make it a thin wrapper that resolves path then
   calls the new function
3. **Refactor tests**: Main tests call the new path-based function directly with
   temp paths, eliminating need for `#[serial]` and env var guards
4. **Acceptable exceptions**: Tests that MUST verify env var behavior (e.g.,
   testing `get_credentials_path()` itself) can keep `#[serial]` and guards -
   move guard structs inside those tests to reduce scope

Benefits:

- Parallel test execution for most tests
- No risk of env var pollution between tests
- Cleaner test code without RAII guard boilerplate

## Completed Work

### Refactor Linux Credential Tests (env var elimination)

Refactored `crates/claude-usage/src/credentials/linux.rs` to eliminate
environment variable mutation in token-reading tests:

- Added `get_token_from_path(&Path)` function for testable core logic
- Converted 4 token-reading tests to use temp paths directly (no env var
  mutation)
- Kept 2 tests with `#[serial]` that legitimately test `get_credentials_path()`
- Moved guard structs into the 2 tests that need them (reduced scope)
- `serial_test` dependency remains (still needed for those 2 tests)

Result: Most tests now run in parallel without risk of env var pollution.

### acd-0ci: Basename Disambiguation Rendering Tests

Added three TestBackend rendering tests to verify directory disambiguation in
the TUI:

- `test_renders_disambiguated_directories_with_parent`: same basename, different
  parents
- `test_renders_unique_basenames_without_parent`: unique basenames show basename
  only
- `test_renders_mixed_unique_and_duplicate_basenames`: mixed scenario

File:
`crates/agent-console-dashboard/src/tui/views/dashboard/tests/rendering.rs`

Tests complement existing unit tests in `disambiguation.rs` by verifying actual
TUI rendering output.
