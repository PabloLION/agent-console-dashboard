# Docs-API Agent Memory

## Documentation Patterns

### Environment Variables

Located at `docs/user/environment-variables.md`. Uses CSV tables for variable
documentation with Variable, Default, and Example columns.

Format:

````markdown
### VARIABLE_NAME

Description of what it controls.

```csv
Variable,Default,Example
VARIABLE_NAME,default_value,"VARIABLE_NAME=value command"
```
````

Additional details...

```

### Function Documentation

Rust doc comments follow rustdoc conventions:
- `///` for public items
- Summary line first, then blank line, then details
- Multi-paragraph explanations for complex behavior
- Reference related functions/messages when helpful

## Issue acd-9n5 Completed

Added documentation for:
1. `ACD_LOG` environment variable startup-only behavior in `docs/user/environment-variables.md`
2. `is_daemon_running()` function doc comment explaining daemon reuse behavior in `crates/agent-console-dashboard/src/commands/daemon.rs`

Both files updated successfully. Tests pass (cargo test --package agent-console-dashboard).
Clippy passes with no warnings.

Ready for orchestrator to commit.

## Pre-existing Test Failure

`claude-usage` crate has a failing test: `client::tests::test_fetch_with_invalid_token`. This is unrelated to documentation work. All agent-console-dashboard tests pass.
```
