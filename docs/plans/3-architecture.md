# Architecture

Technical architecture and design decisions.

---

## System Overview

```text
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│ Claude Session 1│  │ Claude Session 2│  │ Future Agent    │
│     (hooks)     │  │     (hooks)     │  │    (hooks)      │
└────────┬────────┘  └────────┬────────┘  └────────┬────────┘
         │                    │                    │
         └────────────────────┼────────────────────┘
                              │ IPC: update <session> <status>
                              ▼
                    ┌─────────────────────┐
                    │  Agent Console      │
                    │     Daemon          │
                    │                     │
                    │  ┌───────────────┐  │
                    │  │ Session Store │  │  ← in-memory
                    │  │ API Metrics   │  │
                    │  │ State History │  │
                    │  └───────────────┘  │
                    │                     │
                    │  Unix socket:       │
                    │  /tmp/agent-console.sock
                    └──────────┬──────────┘
                               │ IPC: subscribe / query
                    ┌──────────┼──────────┐
                    ▼          ▼          ▼
               ┌───────┐  ┌───────┐  ┌───────┐
               │Tab 1  │  │Tab 2  │  │Tab 3  │
               │display│  │display│  │display│
               └───────┘  └───────┘  └───────┘
```

---

## Architecture Pattern

### What Pattern Do We Use?

Our architecture combines several patterns:

| Pattern           | How we use it                                          |
| ----------------- | ------------------------------------------------------ |
| **Client-Server** | Daemon = server, hooks/dashboards = clients            |
| **Observer**      | Dashboards observe state changes, daemon notifies them |
| **Push model**    | Daemon pushes updates (vs dashboards polling)          |
| **Event-driven**  | Hooks fire events, daemon processes them               |

**Closest named pattern:** Observer pattern (Gang of Four) + Client-Server

```text
┌─────────┐         ┌─────────────────┐
│  Hook   │────────►│                 │
└─────────┘  push   │                 │
┌─────────┐         │     Daemon      │────► Dashboard 1
│  Hook   │────────►│  (Subject/Hub)  │────► Dashboard 2
└─────────┘  push   │                 │────► Dashboard 3
                    └─────────────────┘
                         broadcast
```

### Why Not Pub/Sub?

Pub/sub would be overkill. Here's why:

| Pub/Sub requires                        | Our system                      |
| --------------------------------------- | ------------------------------- |
| Message broker (Redis, RabbitMQ, Kafka) | Daemon IS the hub               |
| Topics/channels for filtering           | No topics - just "all sessions" |
| Message persistence/queuing             | No persistence needed           |
| Decoupled publishers/subscribers        | Direct socket connections       |

### What Our System Is NOT

| Pattern          | Why not                              |
| ---------------- | ------------------------------------ |
| Pub/Sub          | No broker, no topics, no persistence |
| Request-Response | Dashboards subscribe, don't poll     |
| Peer-to-Peer     | Centralized daemon                   |

### Similar Systems

Our architecture resembles:

- **Event bus** (lightweight, single process)
- **Mediator pattern** (daemon mediates between hooks and dashboards)
- **Notification center** (central hub broadcasts to observers)

---

## Data Model

```rust
/// Session state with history
struct Session {
    id: String,
    agent_type: AgentType,       // ClaudeCode, Future agents
    status: Status,
    working_dir: PathBuf,
    since: Instant,              // When status last changed
    history: Vec<StateTransition>,
    api_usage: Option<ApiUsage>,
    closed: bool,                // For resurrection feature
    session_id: Option<String>,  // Claude Code session ID for resume
}

enum Status {
    Working,
    Attention,
    Question,
    Closed,
}

enum AgentType {
    ClaudeCode,
    // Future: Other agents
}

struct StateTransition {
    timestamp: Instant,
    from: Status,
    to: Status,
    duration: Duration,
}

struct ApiUsage {
    input_tokens: u64,
    output_tokens: u64,
    // Extend as needed
}

// In-memory store
type Store = HashMap<String, Session>;
```

---

## Backend Architecture Decision

**Status:** DECIDED
**Decision:** Single Daemon
**Date:** 2026-01-17

### Decision Rationale

Evaluated three approaches: Single Daemon, Shared Memory, SQLite.

**Why Daemon:**

- **Minimal footprint** - One socket file, no database
- **Real-time updates** - Push model, no polling
- **Volatile state fits** - Sessions are transient; persistence not needed
- **Safe Rust** - No `unsafe` code required (unlike shared memory)
- **Simple data model** - HashMap in memory, no SQL schema

**Why not Shared Memory:**

- Requires `unsafe` Rust (breaks safety guarantees)
- Platform-specific (POSIX vs Windows)
- Complex synchronization
- Data must be "plain old data" (no Vec, String)

**Why not SQLite:**

- Adds 1-2MB to binary size
- Requires polling for updates (~100ms latency)
- Persistence not needed for volatile session state
- Schema/migrations overhead for simple key-value data

**Crash handling:**

- Daemon crash = state lost (acceptable for volatile state)
- Hooks re-register on next event
- Sessions refresh quickly through normal user interaction

### Auto-start Behavior

Daemon auto-starts if not running when client connects. First hook or dashboard that runs starts the daemon automatically.

---

## IPC Protocol

Text-based protocol over Unix socket:

```text
# Commands (client → daemon)
SET <session> <status> [metadata_json]
RM <session>
LIST
SUBSCRIBE
RESURRECT <session>
API_USAGE <session> <tokens_json>

# Responses (daemon → client)
OK
OK <data_json>
ERR <message>
STATE <json>
UPDATE <session> <status> <elapsed_seconds>
```

---

## CLI Interface

### Daemon Mode

```bash
# Start daemon (foreground, for development)
agent-console daemon

# Start daemon (background)
agent-console daemon --daemonize

# With custom socket path
agent-console daemon --socket /tmp/agent-console.sock
```

### Client Commands

```bash
# Update session status (called by hooks)
agent-console set <session> working
agent-console set <session> attention
agent-console set <session> question

# Remove session
agent-console rm <session>

# Query all sessions (one-shot)
agent-console list

# Subscribe to updates (streaming)
agent-console watch

# Resurrect closed session
agent-console resurrect <session>

# Report API usage
agent-console api-usage <session> --input 1000 --output 500
```

### TUI Dashboard

```bash
# Interactive dashboard (connects to daemon)
agent-console tui

# With specific layout
agent-console tui --layout two-line
```

---

## Project Structure

```text
agent-console/
├── Cargo.toml
├── src/
│   ├── main.rs           # CLI entry, argument parsing
│   ├── daemon/
│   │   ├── mod.rs
│   │   ├── server.rs     # Socket server
│   │   ├── store.rs      # State management
│   │   └── protocol.rs   # IPC message parsing
│   ├── client/
│   │   ├── mod.rs
│   │   └── commands.rs   # CLI client commands
│   ├── tui/
│   │   ├── mod.rs
│   │   ├── app.rs        # Application state
│   │   ├── widgets/      # Widget implementations
│   │   └── layouts.rs    # Layout presets
│   ├── config.rs         # Configuration parsing
│   └── lib.rs            # Shared types
├── config/
│   └── default.toml      # Default configuration
└── hooks/
    └── claude-code/      # Example hooks for Claude Code
```

---

## Dependencies

| Crate       | Purpose               |
| ----------- | --------------------- |
| tokio       | Async runtime         |
| ratatui     | Terminal UI framework |
| clap        | CLI argument parsing  |
| serde       | Serialization         |
| serde_json  | JSON handling         |
| toml        | Config file parsing   |
| directories | XDG paths             |

---

## Success Metrics

| Metric         | Target |
| -------------- | ------ |
| RAM usage      | <5MB   |
| Update latency | <1ms   |
| Binary size    | <10MB  |
| Startup time   | <100ms |
