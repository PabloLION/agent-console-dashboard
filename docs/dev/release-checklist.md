# Release Checklist

Steps to complete before each minor version bump.

## Quality Gates

- [ ] `cargo test --workspace` — all tests pass
- [ ] `cargo clippy --workspace -- -D warnings` — no warnings
- [ ] `cargo doc --workspace --no-deps` — docs build clean

## Memory Audit

Run biweekly (each minor version bump):

- [ ] Read all agent memories (`.claude/agent-memory/*/MEMORY.md`)
- [ ] Compare against `docs/decisions/INDEX.md`
- [ ] Create missing decision docs for any gaps
- [ ] Fix any collisions (memory contradicts doc)
- [ ] Update `docs/decisions/INDEX.md` with new entries

See acd-2x35 for the full audit process.

## Version Bump

- [ ] Update `Cargo.toml` version
- [ ] `cargo install --path crates/agent-console-dashboard`
- [ ] Verify `acd --version` shows new version
- [ ] `bd sync && git push`
