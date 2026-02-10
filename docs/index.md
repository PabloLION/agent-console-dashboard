# Documentation Index

Map of the docs/ folder. Start here to find what you need.

## Folder Structure

### design/ — Why we build what we build

Product vision, principles, and goals. Read these first to understand the
project's purpose.

- [vision.md](design/vision.md) — Core purpose, origin, design principles,
  non-goals, tech stack
- [ui.md](design/ui.md) — UI philosophy, widget model, session statuses,
  layouts, navigation
- [integrations.md](design/integrations.md) — Hook-based push model, Claude Code
  hooks, multiplexer support, API integration

### architecture/ — How pieces fit together

Implementation-level structure: components, data flow, concurrency.

- [concurrency.md](architecture/concurrency.md) — Actor model, mpsc channel,
  single-threaded event loop
- [error-handling.md](architecture/error-handling.md) — Error flow by component,
  retry strategy, TUI error display
- [hooks.md](architecture/hooks.md) — All 6 hooks, plugin manifest, install/
  uninstall commands, troubleshooting
- [widget-data-flow.md](architecture/widget-data-flow.md) — Centralized data
  flow, WidgetContext, daemon as single source of truth

### decisions/ — Why X over Y

Architectural decisions with context, rationale, and alternatives considered.
Each file answers "why did we choose this approach?"

- [backend-architecture.md](decisions/backend-architecture.md) — Daemon + TUI
  split, why not monolith
- [concurrency-model.md](decisions/concurrency-model.md) — Actor model over
  RwLock
- [ipc-protocol.md](decisions/ipc-protocol.md) — Line-delimited JSON over Unix
  socket
- [session-identification.md](decisions/session-identification.md) — session_id
  from hook JSON stdin
- [session-lifecycle.md](decisions/session-lifecycle.md) — Status transitions,
  closed session handling
- [variable-naming.md](decisions/variable-naming.md) — Naming rationale for
  types and structs (SessionSnapshot, etc.)
- [credential-storage.md](decisions/credential-storage.md) — Keychain + file
  fallback
- [keychain-access-method.md](decisions/keychain-access-method.md) — Why
  `/usr/bin/security` CLI over `security-framework` crate (macOS ACL)
- [hook-contract.md](decisions/hook-contract.md) — Fire-and-forget, exit codes,
  JSON stdin/stdout
- [hook-installation.md](decisions/hook-installation.md) — Plugin system over
  manual settings.json
- [hook-stdin-data.md](decisions/hook-stdin-data.md) — Which fields to parse
  from hook JSON
- [auto-stop.md](decisions/auto-stop.md) — Daemon idle auto-stop behavior
- [idle-auto-stop.md](decisions/idle-auto-stop.md) — Idle detection specifics
- [error-propagation.md](decisions/error-propagation.md) — Daemon-to-TUI error
  broadcast
- [socket-and-cleanup.md](decisions/socket-and-cleanup.md) — Socket path, stale
  socket cleanup
- [widget-data-source.md](decisions/widget-data-source.md) — Daemon fetches
  centrally, TUI is stateless
- [config-and-reload.md](decisions/config-and-reload.md) — TOML config, hot
  reload behavior
- [retry-and-connection.md](decisions/retry-and-connection.md) — TUI reconnect
  backoff, hook retry policy
- [workspace-structure.md](decisions/workspace-structure.md) — Cargo workspace
  layout
- [testing-strategy.md](decisions/testing-strategy.md) — Test organization and
  conventions
- [complexity-review.md](decisions/complexity-review.md) — Scope review and
  simplification decisions
- [implementation-defaults.md](decisions/implementation-defaults.md) — Grouped
  small defaults (channel sizes, timeouts, etc.)
- [deferred.md](decisions/deferred.md) — Explicitly deferred decisions with
  rationale

### epics/ — Active work packages

- [tui-polish.md](epics/tui-polish.md) — Current TUI improvement epic

### archive/ — Historical artifacts

Planning documents, epic/story specs, and audits from the original design phase.
Preserved for reference but not maintained. Cross-references within archive may
be stale.

- `planning/` — Original plans (0-index through 7-decisions), discussion records
- `epic/` — E001-E014 epic specifications
- `stories/` — S001-S014 story specifications
- `audits/` — BMAD validation, content audit, structural audit, alignment review
- `scripts/` — Story validation script

### Root files

- [project-status.md](project-status.md) — Current implementation status and
  progress
- [end-to-end-blockers.md](end-to-end-blockers.md) — Blockers for end-to-end
  functionality
- [terminology.md](terminology.md) — Project-specific term definitions
