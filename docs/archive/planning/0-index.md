# Agent Console Dashboard

**Project Status:** Planning Complete ✓ **Repository:**
[agent-console-dashboard](https://github.com/pablolion/agent-console-dashboard)
**Renamed from:** CC-Hub **Created:** 2026-01-16

---

## Versioning

| Version | Status      | Scope                                           |
| ------- | ----------- | ----------------------------------------------- |
| v0      | **Current** | Proof of concept, file-based, usage API         |
| v1      | Planned     | Daemon backend, session tracking, full features |

---

## Documents

| #   | Document                              | Description                                    |
| --- | ------------------------------------- | ---------------------------------------------- |
| 1   | [History](1-history.md)               | Evolution from CC-Hub, existing implementation |
| 2   | [Features](2-features.md)             | Complete feature list with priorities          |
| 3   | [Architecture](3-architecture.md)     | Technical decisions and system design          |
| 4   | [UI Design](4-ui-design.md)           | Widget-based UI system                         |
| 5   | [Integrations](5-integrations.md)     | Zellij, hooks, multiplexer support             |
| 6   | [Open Questions](6-open-questions.md) | Unresolved decisions                           |
| 7   | [Decisions](7-decisions.md)           | Decision log with detailed rationale           |

---

## Project Identity

**Agent Console Dashboard** is a terminal-based dashboard for managing multiple
AI coding agent sessions.

### Core Purpose

- Track multiple Claude Code sessions across terminal panes
- Show which sessions need user attention
- Display API usage and session state
- Resurrect/reopen closed sessions
- Support extensibility for other agents

### Why Rename from CC-Hub?

1. **Broader scope** - Not limited to Claude Code, will support other agents
   with hooks
2. **More features** - API monitoring, session history, resurrection
3. **Professional identity** - "Agent Console" describes the product better

---

## Tech Stack

| Layer         | Technology | Notes                      |
| ------------- | ---------- | -------------------------- |
| Language      | Rust       | Performance, single binary |
| Text UI       | Ratatui    | Terminal UI framework      |
| Async         | Tokio      | Async runtime for daemon   |
| CLI           | Clap       | Argument parsing           |
| Serialization | Serde      | JSON for config and IPC    |
| Backend       | TBD        | See architecture doc       |

---

## Quick Links

- Original CC-Hub docs merged into this project
- Related: Claude Code status line widget (separate project, shares UI concepts)
