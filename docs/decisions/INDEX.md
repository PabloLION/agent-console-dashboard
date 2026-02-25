# Decision Index

One-line summary of every decision document in this directory, sorted
alphabetically by filename.

- **activation-gestures.md**: Enter and double-click both fire the configured
  hook; Enter does not expand the detail view; Esc and header click deselect;
  scroll always navigates the session list
- **auto-stop.md**: Auto-stop the daemon after 60 minutes idle when no
  dashboards are connected and no active sessions exist
- **backend-architecture.md**: Use a single daemon process with in-memory
  HashMap over SQLite or POSIX shared memory
- **backup-naming.md**: Place `.bak` at the end of backup filenames with the
  timestamp in the middle (e.g., `config.toml.20260223T112407Z.bak`)
- **click-detection.md**: Store the session list's rendered `Rect` on `App`
  during the render pass instead of computing offsets with magic numbers
- **complexity-review.md**: Audit unused types and duplicate methods in the
  codebase (pending review, not a resolved decision)
- **concurrency-model.md**: Use a single-threaded actor model with one mpsc
  channel and event loop over `tokio::spawn` per connection with
  `Arc<RwLock<HashMap>>`
- **config-and-reload.md**: Store config at
  `~/.config/agent-console/config.toml`, support zero-config first run, and
  allow SIGHUP hot reload for most settings
- **credential-storage.md**: Reuse Claude Code's existing OAuth credentials via
  platform-specific retrieval (macOS Keychain via `/usr/bin/security`, Linux
  JSON file)
- **deferred.md**: Document intentionally deferred features — Zellij/tmux
  plugins, Windows support, man pages, and sound notifications are all deferred
  to v1+ or v2+
- **detail-panel-layout.md**: The detail panel is always visible as a 12-line
  fixed section below the session list; it does not toggle; hint text appears
  when nothing is selected
- **error-propagation.md**: Broadcast daemon errors to connected TUI dashboards
  rather than to Claude, with hooks treated as fire-and-forget
- **history-display-format.md**: Status history shows per-state duration (e.g.,
  "5m32s working → attention"), not wall-clock timestamps, to make dwell time
  immediately readable
- **hook-contract.md**: ACD hooks never exit with code 2 (which would block
  Claude), always exiting 0 or 1, with a 5-second timeout
- **hook-field-type.md**: Use `Vec<HookConfig>` for `activate_hooks` and
  `reopen_hooks` instead of `Option<String>`, matching Claude Code's own hook
  structure
- **hook-installation.md**: Use an idempotent append algorithm for
  `acd hooks install` that preserves existing user hooks
- **hook-stdin-data.md**: Parse Claude Code's JSON stdin in `acd set` using a
  `--source` flag to select the parser, enabling future multi-agent support
- **idle-auto-stop.md**: Add a periodic idle check in the main event loop that
  triggers graceful shutdown after 60 minutes with no active sessions
- **implementation-defaults.md**: Group of small standalone decisions — SIGTERM
  graceful shutdown, `0600` socket permissions, `clap_complete` shell
  completions, pinned Rust toolchain for CI reproducibility, and others
- **ipc-protocol.md**: Use JSON Lines (newline-delimited JSON) over Unix socket
  for IPC, with `SessionSnapshot` as the canonical wire struct; `agent_type`
  serializes as `"claudecode"` (lowercase Debug format)
- **keychain-access-method.md**: Shell out to `/usr/bin/security` to read Claude
  Code's macOS Keychain credentials instead of using the `security-framework`
  crate directly, because only `/usr/bin/security` is in the item's ACL
- **post-merge-hook.md**: Run `cargo fmt --check` and `cargo test` automatically
  in a `scripts/post-merge.sh` hook to catch formatting drift from agent
  worktrees
- **pre-commit-hooks.md**: Auto-fix and re-stage with `cargo fmt` in pre-commit,
  but keep `cargo clippy` as a report-only gate without auto-fix
- **resurrect-to-reopen.md**: Replace the `acd resurrect` CLI command and
  built-in multiplexer support with configurable `reopen_hooks`, renaming the
  concept to "reopen" throughout
- **retry-and-connection.md**: Poll for the socket file after auto-starting the
  daemon rather than using fixed sleep delays, and show stale API data with age
  indicators on failure
- **session-identification.md**: Use Claude Code's `session_id` from JSON stdin
  as the primary session identifier, sending the full payload on every hook call
  with no separate registration step
- **session-lifecycle.md**: Track four session statuses (Working, Attention,
  Question, Closed), detect closure via the SessionEnd hook only, and retain
  closed sessions in a bounded history
- **session-sorting.md**: Sort sessions by status group ascending, then priority
  descending, then elapsed time descending
- **session-update-command.md**: Use a single `acd session update <id>` command
  with optional flags over separate per-field commands, for atomic multi-field
  updates
- **shell-execution.md**: Execute user-configured commands via `sh -c` to handle
  word splitting, and pass data through environment variables rather than string
  substitution to avoid shell injection
- **socket-and-cleanup.md**: Use `$XDG_RUNTIME_DIR/acd.sock` on Linux and
  `$TMPDIR/acd.sock` on macOS, cleaning up the socket file on all shutdown paths
- **status-symbols.md**: Status symbols are ASCII characters (`*`, `!`, `?`,
  `x`, `.`) chosen over Unicode alternatives for reliable rendering across SSH
  and older terminal emulators
- **task-runner-choice.md**: Use shell scripts in `scripts/` over `just` or
  `cargo xtask` because they require no external dependencies and the tasks are
  simple cargo wrappers
- **testing-strategy.md**: Place unit tests inside source files with
  `#[cfg(test)]`, reserve `tests/` for public API integration tests, and use
  `#[ignore]` with name prefixes (`net_`, `env_`, `svc_`) for tests requiring
  external resources
- **variable-naming.md**: Name the IPC wire struct `SessionSnapshot` to clearly
  communicate a frozen, computed point-in-time view distinct from the live
  `Session` struct
- **version-display.md**: Display the version string right-aligned in the TUI
  header row (moved from footer in acd-mq6y); footer bottom-right reserved for
  API usage
- **widget-data-source.md**: Route all widget data through the daemon as the
  single source of truth, with the daemon fetching API usage every 3 minutes and
  broadcasting to subscribed TUI clients
- **workspace-structure.md**: Use a Cargo workspace with two crates —
  `agent-console-dashboard` (binary) and `claude-usage` (publishable library)
- **xdg-directory-selection.md**: Use `XDG_STATE_HOME` for logs and history (not
  `XDG_DATA_HOME`), with a `state_dir().or_else(data_dir)` fallback chain for
  macOS
