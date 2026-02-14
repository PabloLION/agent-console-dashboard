# Build Tooling Agent Memory

## Rust Toolchain Pinning (acd-c1h)

**Problem:** CI lint failures from rustfmt version mismatch between local and CI
environments.

**Solution:** Create `rust-toolchain.toml` at repo root:

- Pin to specific Rust version (1.93.1 for this project)
- Include components: `rustfmt`, `clippy`
- Both local rustup and `dtolnay/rust-toolchain@stable` in CI auto-read this
  file
- No CI workflow changes needed — pinning happens automatically

**Format:**

```toml
[toolchain]
channel = "1.93.1"
components = ["rustfmt", "clippy"]
```

**Effect:** Ensures identical rustfmt behavior locally and in CI, eliminating
version-drift lint failures.

## Flaky Network Tests

**Pattern:** Tests making real HTTP requests should be marked `#[ignore]` with
network reason.

**Example:** `test_fetch_with_invalid_token` in
`crates/claude-usage/src/client.rs`

- Makes real API call to Anthropic
- Fails intermittently with `Network` or `RateLimited` instead of expected
  `Unauthorized`
- Fix: `#[ignore = "requires network access to Anthropic API"]`
- Same pattern as other integration tests in the file

## Pre-commit Hook Test Output Filtering (acd-zhq)

**Problem:** `cargo test --workspace --quiet` produces 81 lines of output,
mostly test progress dots (625+ individual test indicators like "......." or
"... 174/625").

**Solution:** Filter test output in `scripts/pre-commit.sh`:

- Capture test output to temp file
- On success: filter lines starting with dots using `grep -v '^\.\+'`
- On failure: show full unfiltered output for debugging
- Always cleanup temp file

**Key insight:** Test harness offers only terse (dots), pretty (names), or json
(nightly) formats. The `--quiet` flag reduces cargo build verbosity but not test
progress dots. Shell-level filtering is the only option.

**What gets filtered:** Any line starting with one or more dots

- Pure dot lines: "..........................."
- Dot progress lines: "........................... 174/625"
- Dot error lines: "...error: Unrecognized..." (inline test output)

**What gets preserved:** Lines that don't start with dots

- "warning:" messages
- "running N tests" headers
- "test result:" summaries
- "error:" compilation errors (cargo errors, not test dots)

**Pattern:** `^\.\+` matches start of line + one or more dots

**Testing:** Orchestrator must verify by running the pre-commit hook with staged
rust files. Expected output: ~5-10 lines (headers + summaries only) instead
of 81.
