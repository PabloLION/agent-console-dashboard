# claude-hooks Design Document

## Version Roadmap

| Version | Features | Status |
|---------|----------|--------|
| **v0.1** | install, uninstall, list (user scope only), registry, atomic writes — library only | To implement |
| **v0.2** | Multi-scope (user/project/local), export, import, CLI binary | Designed |
| **v1.0** | All of v0.2 + doctor | Designed |
| **Post-v1** | Everything else | Deferred |

## Feature Matrix

| Feature | v0.1 | v0.2 | v1.0 | Post-v1 | Notes |
|---------|------|------|------|---------|-------|
| install | ✅ | ✅ | ✅ | ✅ | User scope only in v0.1 |
| uninstall | ✅ | ✅ | ✅ | ✅ | User scope only in v0.1 |
| list | ✅ | ✅ | ✅ | ✅ | Shows managed vs unmanaged |
| registry | ✅ | ✅ | ✅ | ✅ | Simple JSON in XDG data dir |
| atomic writes | ✅ | ✅ | ✅ | ✅ | Write-temp-then-rename |
| CLI binary | ❌ | ✅ | ✅ | ✅ | Standalone `claude-hooks` command (v0.2+) |
| multi-scope | ❌ | ✅ | ✅ | ✅ | user/project/local |
| export | ❌ | ✅ | ✅ | ✅ | Agent representation format |
| import | ❌ | ✅ | ✅ | ✅ | Agent representation format |
| doctor | ❌ | ❌ | ✅ | ✅ | Diagnose sync issues |
| migrate | ❌ | ❌ | ❌ | ✅ | Can use export+import instead |
| enable/disable | ❌ | ❌ | ❌ | ✅ | Can use uninstall+reinstall instead |
| show | ❌ | ❌ | ❌ | ✅ | list is sufficient |
| status | ❌ | ❌ | ❌ | ✅ | list is sufficient |
| diff | ❌ | ❌ | ❌ | ✅ | Manual comparison is fine |
| repair | ❌ | ❌ | ❌ | ✅ | Edge case |
| update | ❌ | ❌ | ❌ | ✅ | Use uninstall+install |
| dry-run | ❌ | ❌ | ❌ | ✅ | Nice-to-have |
| templates/catalog | ❌ | ❌ | ❌ | ✅ | Different product |
| hook versioning | ❌ | ❌ | ❌ | ✅ | Over-engineering |
| universal format | ❌ | ❌ | ❌ | ✅ | Different product |
| cross-agent | ❌ | ❌ | ❌ | ✅ | Different product |
| shell completions | ❌ | ❌ | ❌ | ✅ | Nice-to-have |
| --json output | ❌ | ❌ | ❌ | ✅ | Nice-to-have |

## Non-Goals (Explicitly Out of Scope)

These are **not** planned for claude-hooks, even in future versions:

1. **Package manager for hooks** — We are a settings.json editor, not npm for hooks. No dependency resolution, no registry server, no versioned packages.

2. **Cross-agent hook transfer** — Each agent (Claude, Codex, Gemini) should have its own adapter crate. agent-console-dashboard coordinates them. claude-hooks only handles Claude Code.

3. **Functional validation of hooks** — We cannot execute hooks to test them. We only do structural validation (file exists, executable). Callers are responsible for ensuring their hooks work.

4. **Hook templating system** — No built-in catalog of hook templates. Callers (like ACD) provide their own hook definitions.

5. **Automatic hook updates** — No mechanism to detect and update outdated hooks. Users uninstall old + install new.

6. **Config file for claude-hooks itself** — Zero-config. All behavior via CLI flags.

## Decisions (Reference)

The decisions below were made during design discussions. Many apply to features deferred to post-v1. They are preserved as reference for future implementation.

### D01: File write strategy

**Q**: How do we safely modify `settings.json`?
**A**: Atomic rename pattern. Write to temp file in same directory, fsync, rename over original. On error, leave original untouched and print path to the safety copy.

### D02: Backup strategy

**Q**: Do we maintain backups of settings.json?
**A**: No managed backups. Safety copies are created during writes and cleaned up on success. On failure, the safety copy is preserved and user is directed to it. No rotation, no git, no dedicated backup directory.

### D03: Timestamp format

**Q**: What timestamp format for any generated filenames?
**A**: `yyyyMMdd-hhmmss` (17 chars with dash). Example: `20260202-143022`.

### D04: Import/export over backup/restore

**Q**: Should we have backup/restore?
**A**: No. Import/export replaces backup/restore. Export writes hook definitions to a file. Import reads them back. The user manages exported files however they want (git, cloud sync, etc.).

### D05: Two representations

**Q**: What format do we use for hook definitions?
**A**: Two representations exist:
- **Agent representation**: matches the source agent's native format (e.g., Claude's settings.json structure), plus metadata fields (`agent_name`, `agent_version`). Used as default for import/export.
- **Universal representation**: our own JSONC format with richer metadata. Used for cross-agent transfer. Defined later (v0.2+).

Default export format = agent representation (less confusing for users).

### D06: Hook metadata fields

**Q**: What metadata do we store per hook in our representation?
**A**: Dedicated fields (not comments):
- `added_at`: when the hook was registered
- `reason`: why it was added
- `description`: what the hook does
- `optional`: whether the hook is optional or required
Comments in JSONC are reserved for user's own free-form notes.

### D07: Agent version tracking

**Q**: Do we track the source agent version?
**A**: Yes. The agent representation includes `agent_name` (e.g., "claude-code") and `agent_version` (e.g., "1.0.42"). This lives in the agent representation, not the universal one.

### D08: No full package manager

**Q**: Are we building a hook package manager (like npm)?
**A**: No. We are a settings.json editor with import/export. No dependency resolution, no registry, no versioned packages.

### D09: Cross-agent architecture

**Q**: How do hooks transfer between agents (Claude → Codex → Gemini)?
**A**: Each agent has its own adapter crate (`claude-hooks`, `codex-hooks`, etc.). Cross-agent transfer goes through the universal representation. Direct agent-to-agent transfer is not supported — users go through `agent-console-dashboard` as coordinator. Each adapter crate handles only its own agent's format.

### D10: Crate documentation

**Q**: Where do design docs live?
**A**: `crates/claude-hooks/docs/` — design decisions, format spec, architecture notes. Agile plans (epics/stories) also live at crate level.

### D11: JSONC for our files, JSON for theirs

**Q**: What parser do we use?
**A**: `serde_json` for Claude's `settings.json` (standard JSON). A JSONC-capable parser for our own format files. Claude's file is never modified to include comments.

### D12: v0.1 scope

**Q**: What ships in v0.1?
**A**: install, uninstall, list. Our 3 ACD hooks only. User scope only. Atomic writes with safety copy. No import/export yet (but format spec documented).

### D13: settings.json structure (confirmed)

**Q**: What else lives in Claude Code's settings.json besides hooks?
**A**: Top-level keys: `cleanupPeriodDays`, `env`, `permissions`, `hooks`, `statusLine`, `enabledPlugins`, `syntaxHighlightingDisabled`. We only touch `hooks`. Parse as `serde_json::Value` to preserve everything else.

### D14: Migrate between scopes

**Q**: How should scope migration work?
**A**: Move hooks between the 3 Claude Code scopes (global ↔ project ↔ project-local). Atomic: remove from source scope, add to target scope. Note: Claude Code may not support unknown fields in settings.json (JSON schema validation), so our metadata fields only live in exported files, not in settings.json. Migration is a post-v0.1 feature.

### D15: Default export format

**Q**: When exporting, should we default to agent representation or universal?
**A**: Agent representation (matches source agent's format + our metadata). Less confusing for users. Universal format defined in v0.2+.

### D16: Local registry for tracking installed hooks

**Q**: How do we know which hooks we installed?
**A**: Keep a local registry in XDG data directory. When we install hooks, we record what was installed. On uninstall, we match against our registry and only remove exact matches — never touch hooks we didn't install. Users who install ACD hooks manually (without this crate) won't have registry entries, so we can't track those.

### D17: Universal representation is a superset

**Q**: Is the universal format a superset or intersection of agent formats?
**A**: Superset. All fields from all agents are included. Fields not supported by a specific agent are marked as unavailable for that agent. If a hook uses agent-specific features that can't translate, the export notes this.

### D18: Programmatic callers can add custom fields

**Q**: Can other programs using our library add their own metadata?
**A**: Yes. Programs calling our API can specify custom metadata fields (e.g., their software version, purpose). These are stored in the exported representation alongside our standard metadata fields.

### D19: Registry format

**Q**: What format for the local registry?
**A**: JSONC. Consistent with our export format.

### D20: Registry tracks only our hooks

**Q**: Does the registry track all hooks or only ours?
**A**: Only hooks installed through this crate. We don't claim ownership of hooks we didn't install.

### D21: List shows all hooks with ownership markers

**Q**: What does `list` show?
**A**: All hooks from settings.json, with markers indicating which ones are managed by us (matched against registry). Unmanaged hooks shown without metadata.

### D22: Hook identity is a composite key

**Q**: How do we identify a hook uniquely?
**A**: Composite key: `(event, matcher, type, command)`. No separate ID or hash. Identity is derived from content. If any of these fields change, it's a different hook. Behavioral fields (timeout, async, etc.) are configuration, not identity.

### D23: Enable/disable via boolean + removal

**Q**: How does enable/disable work?
**A**: Registry has `enabled: true/false`. Disabled hooks are removed from settings.json but kept in registry with all metadata. Re-enable re-inserts them into settings.json. Designed now, implemented in v0.4.

### D24: installed_by is free-form string

**Q**: How do we identify which program installed a hook?
**A**: Free-form string. e.g., `"acd"`, `"my-tool"`. No enum.

### D25: Scope naming

**Q**: What are the scope names?
**A**: `user` (not "global"), `project`, `local`. Maps to `~/.claude/settings.json`, `.claude/settings.json`, `.claude/settings.local.json`.

### D26: Migrate UX — list then pick

**Q**: How does the user select hooks to migrate?
**A**: `claude-hooks migrate --from user --to project` lists all hooks in source scope with indices. User picks by index (e.g., `1 3 5`). No need to specify event+command manually.

### D27: Migrate atomicity — write both temps, rename target first

**Q**: How do we ensure atomicity when migrate touches two files?
**A**: Write both temp files first (validates both writes succeed). Then rename target temp → target (adding hook). Then rename source temp → source (removing hook). If second rename fails, hook exists in both scopes (recoverable duplicate). If first rename fails, nothing changed.

### D28: Migrate conflict — error by default

**Q**: What if the hook already exists in the target scope?
**A**: Error out. `--force` flag to overwrite.

### D29: Export format — separate hook and metadata

**Q**: How are hooks structured in export files?
**A**: Each entry has two fields: `hook` (agent's native format, as-is) and `metadata` (from our registry). File-level fields: `format`, `format_version`, `agent_name`, `agent_version`, `exported_at`. Per-hook metadata includes `scope`, `added_at`, `description`, `reason`, `optional`, `installed_by`.

### D30: Export format confirmed

```jsonc
{
  "format": "agent",
  "format_version": 1,
  "agent_name": "claude-code",
  "agent_version": "1.0.42",
  "exported_at": "20260202-143022",
  "hooks": [
    {
      "hook": {
        "event": "Stop",
        "matcher": "",
        "type": "command",
        "command": "/path/to/stop.sh $SESSION_ID $ARGS",
        "timeout": 600
      },
      "metadata": {
        "scope": "user",
        "added_at": "20260202-140000",
        "description": "Sets session status to 'attention' on Stop event",
        "reason": "Notify ACD daemon when Claude Code stops",
        "optional": false,
        "installed_by": "acd"
      }
    }
  ]
}
```

### D31: Universal event name normalization

**Q**: How are event names handled in universal representation?
**A**: Normalized. Claude's `"Stop"` → universal `"on_stop"`. Requires a mapping table per agent. Defined in v0.5.

### D32: Hook selection by list index

**Q**: How do users select hooks in interactive commands (migrate, etc.)?
**A**: Numbered list display, selection by index. Simple and unambiguous.

### D33: Crate name available

**Q**: Is `claude-hooks` available on crates.io?
**A**: Yes. Confirmed 2026-02-02.

### D34: Command format is full string

**Q**: Is the `command` field a single file path or a full shell command string?
**A**: Full command string. Can include arguments, interpreter prefix, etc. Example: `python3 /path/to/script.py --verbose`. Identity matching uses the exact command string.

### D35: CLI uses double-dash separator (v0.2+)

**Q**: How does the CLI handle commands with their own arguments?
**A**: Double-dash separator. Everything after `--` is the command.
```sh
claude-hooks install --event Stop --installed-by acd -- /path/to/stop.sh --notify --timeout 30
claude-hooks uninstall --event Stop -- /path/to/stop.sh --notify --timeout 30
```
This avoids shell escaping issues and follows Unix convention.

### D36: v0.1 is library-only

**Q**: Does v0.1 include a CLI binary?
**A**: No. v0.1 is library-only. ACD calls the Rust API directly. CLI binary is added in v0.2.

## Registry Schema (v1)

```jsonc
{
  // claude-hooks registry
  "schema_version": 1,
  "agent_name": "claude-code",
  "hooks": [
    {
      // Identity (composite key)
      "event": "Stop",
      "matcher": "",
      "type": "command",
      "command": "/path/to/stop.sh",

      // Configuration (not part of identity)
      "timeout": 600,

      // Our metadata
      "scope": "user",
      "enabled": true,
      "added_at": "20260202-143022",
      "reason": "Notify ACD daemon when Claude Code stops",
      "description": "Sets session status to 'attention' on Stop event",
      "optional": false,
      "installed_by": "acd",
      "custom": {}
    }
  ]
}
```

## Open Questions (none block v0.1)

### O01: JSONC parser choice

**Q**: Which Rust crate for JSONC parsing?
**A**: TBD. Candidates: `json_comments`, `jsonc-parser`, or strip comments before `serde_json`. Needed for registry file. Research during implementation.

### O03: Hook deduplication across scopes

**Q**: Does Claude Code deduplicate hooks when the same command exists in multiple scopes?
**A**: Unknown. Research needed before v0.2. See scratchpad/hook-deduplication-problem.md.

### O05: Export file location

**Q**: Where do exported files go by default?
**A**: TBD before v0.3. XDG data directory or user-specified path.

### O06: Metadata fields in settings.json

**Q**: Can we add our metadata fields directly into Claude Code's settings.json?
**A**: Unknown. Test before v0.2. For now, metadata only in registry and exported files.

---

## v0.1 Implementation Plan

**Scope**: install, uninstall, list (user scope only), registry, atomic writes. Library only, no CLI.

### Files to Create

```text
crates/claude-hooks/
├── Cargo.toml
├── docs/
│   └── design-draft.md     # This document (decisions + roadmap)
├── src/
│   ├── lib.rs              # Public API: install, uninstall, list
│   ├── error.rs            # Error types (thiserror)
│   ├── types.rs            # HookEvent, HookHandler, RegistryEntry
│   ├── settings.rs         # Read/write Claude's settings.json (atomic)
│   └── registry.rs         # Read/write our JSON registry in XDG data dir
```

### Files to Modify

- `Cargo.toml` (workspace root) — add `crates/claude-hooks` to members
- `crates/agent-console-dashboard/Cargo.toml` — add `claude-hooks = { path = "../claude-hooks" }` dependency

### Implementation Steps

**Step 1: Scaffold crate** [commit]
- Create `Cargo.toml` with author `Pablo LION <36828324+PabloLION@users.noreply.github.com>`
- Add to workspace
- Create `src/lib.rs`
- Create `docs/design-draft.md`
- Verify: `cargo build -p claude-hooks`

**Step 2: Types and errors**
- `error.rs`: `Error` enum with variants for settings/registry read/write failures
- `types.rs`:
  - `HookEvent` enum (12 events)
  - `HookHandler` struct (type, command, matcher, timeout)
  - `RegistryEntry` struct (event, matcher, type, command, installed_by, added_at)

**Step 3: Settings reader/writer**
- `settings.rs`:
  - `settings_path() -> PathBuf` (user scope: `~/.claude/settings.json`)
  - `read_settings() -> Result<Value>`
  - `write_settings_atomic(value) -> Result<()>`
  - `add_hook(value, event, handler) -> Value`
  - `remove_hook(value, event, command) -> Value`

**Step 4: Registry reader/writer**
- `registry.rs`:
  - `registry_path() -> PathBuf` (XDG data dir)
  - `read_registry() -> Result<Vec<RegistryEntry>>`
  - `write_registry(entries) -> Result<()>`

**Step 5: Public API** [commit]
- `lib.rs`:
  - `install(event, handler, installed_by) -> Result<()>`
  - `uninstall(event, command) -> Result<()>`
  - `list() -> Result<Vec<ListEntry>>` (shows managed vs unmanaged)

**Step 6: Tests** [commit]
- Unit tests for settings read/write roundtrip (with tempfile)
- Unit tests for registry read/write
- Integration test: install → list → uninstall → list

**Step 7: Wire into ACD**
- Add dependency
- Verify workspace builds and all tests pass

### Verification

```sh
cargo build -p claude-hooks
cargo test -p claude-hooks
cargo build                    # full workspace
cargo test                     # all 473+ tests still pass
```
