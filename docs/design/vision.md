# Vision

## Core Purpose

Agent Console Dashboard (ACD) is a terminal-based dashboard for managing
multiple AI coding agent sessions. It tracks which sessions need user attention,
displays API usage, and enables session resurrection -- replacing the need to
manually check each terminal pane.

## Origin

ACD evolved from CC-Hub, a set of bash/jq scripts that wrote session status to a
JSON file in `/tmp`. The bash implementation proved the concept but hit
fundamental limits: file I/O on every status check, no shared state, no
real-time updates, and no path to add features like API monitoring or session
resurrection. The Rust rewrite addresses all of these.

See [CC-Hub history](../archive/planning/1-history.md) for the full evolution.

## Principles

- **Push over polling** -- Hooks push status changes to the daemon. The daemon
  never polls agents for state.
- **Daemon as single source of truth** -- All session state lives in one daemon
  process. The TUI and CLI are stateless clients that read from it.
- **Lazy-start over persistent service** -- The daemon spawns on the first hook
  event. No launchd/systemd service, no startup scripts, no background process
  until needed.
- **Volatile state** -- Sessions are ephemeral. State lives in memory, not on
  disk. A reboot clears everything, by design.
- **Minimal footprint** -- Single static binary, no runtime dependencies, no
  database, no network.
- **Widget-based, stateless UI** -- The TUI is a collection of widgets that
  render daemon state. Widgets hold no state of their own.
- **Multiplexer-agnostic** -- Works with Zellij, tmux, or bare terminals. No
  multiplexer lock-in.

## Scale Assumptions

Design target for performance and protocol decisions:

- 100 concurrent sessions (20 active, 80 inactive)
- 100 TUI consumers connected to the daemon simultaneously
- Status history: bounded queue, ~10 transitions per session

At this scale, JSON Lines IPC with full session info per message is lightweight
(~1000 small objects per LIST response).

## Features

### Core (v0)

- **Session tracking** -- Real-time status (working, attention, question,
  closed) via Claude Code hooks
- **API usage display** -- Token consumption and rate limit visibility
- **State history** -- Track status transitions over time (not chat history)
- **Session resurrection** -- Reopen closed sessions via `claude --resume`
- **Centralized config** -- Single TOML config file at the XDG config path
- **Install/uninstall** -- `acd install` writes hooks to Claude Code settings;
  `acd uninstall` removes them

### Later

- **Multi-agent support** -- Any agent that can call a shell script on state
  change
- **Zellij native plugin** (WASM) -- Evaluate after v1
- **Tmux native plugin** -- On request only
- **Theme customization**, **sound notifications**, **dynamic session reorder**

## Non-Goals

| Non-goal                         | Rationale                                                                                                    |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| System service (launchd/systemd) | Lazy-start from hooks replaces a persistent daemon. No service management needed.                            |
| Multi-user support               | Single-user tool. Socket permissions are 0600.                                                               |
| Remote/network access            | Local Unix socket only. Security by design.                                                                  |
| Chat history display             | Privacy concerns, high complexity, out of scope. The dashboard tracks status transitions, not conversations. |
| Persistence across reboot        | State is volatile by design. Sessions are ephemeral.                                                         |
| Complex data model               | Key-value session state is sufficient. No relational data, no query language.                                |
| Windows support (v0)             | Deferred to v2+. Named pipes would replace Unix sockets.                                                     |

## Tech Stack

| Layer         | Technology | Role                              |
| ------------- | ---------- | --------------------------------- |
| Language      | Rust       | Performance, single static binary |
| Terminal UI   | Ratatui    | Widget-based TUI framework        |
| Async runtime | Tokio      | Daemon, IPC, concurrent sessions  |
| CLI           | Clap       | Argument parsing, subcommands     |
| Serialization | Serde      | JSON for IPC, TOML for config     |
