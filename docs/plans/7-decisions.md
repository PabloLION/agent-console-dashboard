# Decision Log

Detailed rationale for architectural and design decisions.

**Created:** 2026-01-17

---

## Q1: Backend Architecture

**Decision:** Single Daemon
**Date:** 2026-01-17
**Status:** Resolved

### Context - Backend Architecture

Evaluated three approaches for state management:

1. **SQLite** - Embedded database
2. **Shared Memory** - Inter-process shared memory
3. **Single Daemon** - Long-running process with Unix socket IPC

### Analysis - Backend Architecture

| Aspect         | SQLite           | Shared Memory   | Daemon       |
| -------------- | ---------------- | --------------- | ------------ |
| Binary size    | +1-2MB           | +0              | +0           |
| Safety         | Safe Rust        | Unsafe required | Safe Rust    |
| Persistence    | Built-in         | None            | None         |
| Real-time      | Polling (~100ms) | Yes             | Push (<10ms) |
| Complexity     | Medium           | High            | Medium       |
| Crash recovery | Automatic        | State lost      | State lost   |

### Decision Rationale - Backend Architecture

**Daemon chosen because:**

- **Minimal footprint** - One socket file, no database file
- **Real-time updates** - Push model eliminates polling
- **Volatile state fits** - Sessions are transient; persistence unnecessary
- **Safe Rust** - No `unsafe` code (unlike shared memory)
- **Simple data model** - HashMap in memory, no SQL schema

**Why not Shared Memory:**

- Requires `unsafe` Rust, breaking safety guarantees
- Platform-specific (POSIX vs Windows APIs)
- Complex synchronization with mutexes
- Data must be fixed-size POD types (no Vec, String)

**Why not SQLite:**

- Adds 1-2MB to binary size
- Requires polling for updates
- Schema/migrations overhead for simple key-value data
- Persistence not needed for volatile session state

### Crash Handling - Backend Architecture

- Daemon crash = state lost (acceptable)
- Hooks re-register on next event
- Sessions refresh quickly through normal user interaction

---

## Q2: Daemon Auto-Start

**Decision:** Auto-start if socket doesn't exist
**Date:** 2026-01-17
**Status:** Resolved

### Context - Daemon Auto-Start

When client (hook or dashboard) connects, what if daemon isn't running?

### Options Considered - Daemon Auto-Start

1. **Manual start** - User must run `agent-console daemon` explicitly
2. **Auto-start** - Client starts daemon if socket missing
3. **Hybrid** - Auto-start foreground, manual for background

### Decision Rationale - Daemon Auto-Start

**Auto-start chosen because:**

- Zero configuration required
- First hook or dashboard that runs starts daemon
- Matches behavior of tools like Docker daemon
- No systemd/launchd setup needed for basic usage

### Implementation - Daemon Auto-Start

```rust
// Pseudocode
fn connect_to_daemon() -> Result<Connection> {
    match UnixStream::connect(SOCKET_PATH) {
        Ok(conn) => Ok(conn),
        Err(_) => {
            // Socket doesn't exist, start daemon
            start_daemon_process()?;
            // Retry connection
            UnixStream::connect(SOCKET_PATH)
        }
    }
}
```

---

## Q3: Config File Location

**Decision:** XDG standard (`~/.config/agent-console/`)
**Date:** 2026-01-17
**Status:** Resolved

### Context - Config File Location

Where should configuration files live?

### Options Considered - Config File Location

1. `~/.config/agent-console/config.toml` (XDG)
2. `~/.agent-console.toml` (home directory)
3. `~/.agent-console/config.toml` (custom directory)

### Decision Rationale - Config File Location

**XDG chosen because:**

- Standard on Linux/macOS
- Keeps `$HOME` clean (no dotfile clutter)
- Users know where to look
- `directories` crate handles this automatically

### XDG Specification - Config File Location

| Type   | Variable           | Default           | Our Use                               |
| ------ | ------------------ | ----------------- | ------------------------------------- |
| Config | `$XDG_CONFIG_HOME` | `~/.config/`      | `~/.config/agent-console/config.toml` |
| Data   | `$XDG_DATA_HOME`   | `~/.local/share/` | Not needed (volatile state)           |
| Cache  | `$XDG_CACHE_HOME`  | `~/.cache/`       | Not needed                            |

---

## Q4: Session Status Types

**Decision:** Four statuses as C-like enum
**Date:** 2026-01-17
**Status:** Resolved

### Context - Session Status Types

What session statuses should we track?

### Statuses Defined - Session Status Types

```rust
/// C-like enum (all unit variants, no associated data)
enum Status {
    Working,    // Agent is processing
    Attention,  // Agent needs user attention
    Question,   // Agent asked a question (AskUserQuestion)
    Closed,     // Session ended
}
```

### Potential Additions Deferred - Session Status Types

- Error (agent crashed)
- Paused (user paused session)
- Rate Limited (API limit hit)

**Deferred because:** Keep v0 simple. Can extend enum later without breaking changes.

### Rust Terminology - Session Status Types

| Term         | Description                    | Example               |
| ------------ | ------------------------------ | --------------------- |
| Enum         | The type itself                | `enum Status { ... }` |
| Unit variant | Variant with no data           | `Working`, `Closed`   |
| C-like enum  | All variants are unit variants | Our `Status` enum     |

---

## Q5: Session Resurrection Scope

**Decision:** Configurable TTL + manual removal
**Date:** 2026-01-17
**Status:** Resolved

### Context - Session Resurrection Scope

How long do we keep closed sessions for resurrection?

### Options Considered - Session Resurrection Scope

1. **Ephemeral** - Until daemon restart
2. **Configurable TTL** - Time limit (e.g., 24 hours)
3. **Forever** - Until manually removed

### Decision Rationale - Session Resurrection Scope

**Configurable TTL + manual removal chosen because:**

- Flexibility: users can set retention period
- Automatic cleanup prevents unbounded memory growth
- Manual removal gives control for specific sessions
- Config example:

```toml
[sessions]
# Keep closed sessions for 24 hours
resurrection_ttl = "24h"
```

### Implementation Notes - Session Resurrection Scope

- Daemon checks TTL periodically (e.g., every 5 minutes)
- CLI command: `agent-console rm <session>` for manual removal
- List closed sessions: `agent-console list --closed`

---

## Q6: API Usage Source

**Decision:** Anthropic Usage API (v0 core feature)
**Date:** 2026-01-17
**Status:** Resolved

### Context - API Usage Source

How do we get token usage data for display?

### Options Considered - API Usage Source

1. **Anthropic Usage API** - Official endpoint
2. **Log parsing** - Parse Claude Code output
3. **Estimate from messages** - Count messages
4. **Per-response tokens** - From hook data

### Decision Rationale - API Usage Source

**Anthropic Usage API chosen because:**

- Official, reliable data source
- Endpoint: `/v1/organizations/usage_report/messages`
- Tracks: input tokens, cached tokens, output tokens
- Can group by: API key, workspace, model

**This is v0 core feature** - without usage display, we fail to prove the concept.

### Authentication - API Usage Source

- Requires Admin API key (`sk-ant-admin...`)
- Only org admins can create Admin API keys
- Details to be investigated during implementation

### References - API Usage Source

- [Usage and Cost API - Claude Docs](https://docs.anthropic.com/en/api/usage-cost-api)
- [Console Usage Reporting](https://support.anthropic.com/en/articles/9534590-cost-and-usage-reporting-in-console)

---

## Q7: AskUserQuestion Hook Detection

**Decision:** Use PreToolUse hook with AskUserQuestion matcher
**Date:** 2026-01-17
**Status:** Resolved

### Context - AskUserQuestion Hook Detection

Can we detect when Claude Code asks the user a question?

### Investigation Summary - AskUserQuestion Hook Detection

Initial research suggested AskUserQuestion was not hookable. Further investigation revealed:

**AskUserQuestion CAN be detected via PreToolUse** (since v2.0.76)

### What Works - AskUserQuestion Hook Detection

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "AskUserQuestion",
        "hooks": [
          {
            "type": "command",
            "command": "agent-console set $SESSION question"
          }
        ]
      }
    ]
  }
}
```

### What Doesn't Work - AskUserQuestion Hook Detection

- Answering questions programmatically (must use CLI)
- AskUserQuestion not listed in official matcher docs (but works)

### Historical Bug (Fixed) - AskUserQuestion Hook Detection

**Issue [#13439](https://github.com/anthropics/claude-code/issues/13439):** PreToolUse hooks caused AskUserQuestion to return empty responses.

**Root cause:** stdin/stdout conflict between hook JSON processing and interactive input.

**Fixed in:** Claude Code v2.0.76 (January 4, 2026)

### Open Bug - AskUserQuestion Hook Detection

**Issue [#15400](https://github.com/anthropics/claude-code/issues/15400):** PermissionRequest hook incorrectly interferes with AskUserQuestion.

### Related Feature Requests - AskUserQuestion Hook Detection

| Issue                                                            | Status | Description                        |
| ---------------------------------------------------------------- | ------ | ---------------------------------- |
| [#15872](https://github.com/anthropics/claude-code/issues/15872) | Open   | Add hook support for notifications |
| [#12605](https://github.com/anthropics/claude-code/issues/12605) | Closed | Documents working workaround       |
| [#10168](https://github.com/anthropics/claude-code/issues/10168) | Open   | Generic UserInputRequired hook     |

### Implementation for Agent Console - AskUserQuestion Hook Detection

Hook configuration for Claude Code:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "AskUserQuestion",
        "hooks": [
          {
            "type": "command",
            "command": "/path/to/agent-console set $CC_SESSION_ID question"
          }
        ]
      }
    ],
    "Notification": [
      {
        "matcher": "elicitation_dialog",
        "hooks": [
          {
            "type": "command",
            "command": "/path/to/agent-console notify elicitation"
          }
        ]
      }
    ]
  }
}
```

### Minimum Version Requirement - AskUserQuestion Hook Detection

**Claude Code v2.0.76 or later** required for reliable AskUserQuestion hook detection.

---

## Q8: Zellij Plugin

**Decision:** Deferred to v2
**Date:** 2026-01-17
**Status:** Deferred

### Context - Zellij Plugin

Should we build a native Zellij plugin (WASM)?

### Decision - Zellij Plugin

Not for v0 or v1. Evaluate for v2.

### Rationale - Zellij Plugin

- WASM adds complexity
- Maintenance burden
- Zellij-specific (not portable)
- Hook-based approach works for v0/v1

---

## Q9: Tmux Plugin

**Decision:** On request only
**Date:** 2026-01-17
**Status:** Deferred

### Context - Tmux Plugin

Should we build a Tmux plugin?

### Decision - Tmux Plugin

Will not implement unless users request it.

### Rationale - Tmux Plugin

- Lower priority than Zellij (user preference)
- Hook-based approach works without plugins
- Defer to v3 or beyond, only if requested

---

## Q10: Default Layout

**Decision:** Three widgets, config file only
**Date:** 2026-01-17
**Status:** Resolved

### Widget Types - Default Layout

| Widget                    | Lines | Description                                        |
| ------------------------- | ----- | -------------------------------------------------- |
| Session Status            | 1     | Compact: `proj-a: - \| proj-b: 2m34s \| proj-c: ?` |
| Expandable Session Status | 2     | Clickable, shows history when expanded             |
| API Usage                 | 1     | Token info (details TBD)                           |

### Configuration - Default Layout

- Layout defined in **config file only**
- No `--layout` CLI argument for v0

### Optional Feature (On Request) - Default Layout

Allow named layouts in config file, selectable via `--layout <name>` CLI arg.

```toml
# Potential future feature
[ui.layouts.minimal]
widgets = ["session-status"]

[ui.layouts.full]
widgets = ["expandable-session-status", "api-usage"]
```

**Status:** Will implement only if requested.

---

## Q11: TUI Framework Mode

**Decision:** Immediate mode
**Date:** 2026-01-17
**Status:** Resolved

### Context - TUI Framework Mode

Ratatui supports two rendering approaches:

- **Immediate mode** - Redraw entire UI each frame
- **Retained mode** - Components persist, only update changes

### Decision Rationale - TUI Framework Mode

**Immediate mode chosen because:**

- Simple mental model
- Standard Ratatui idiom
- Our UI is small (3 widgets)
- Update frequency is low (~1/second for elapsed time)
- No animations needed
- Retained mode benefits (caching, partial updates) don't apply

Retained mode shines at 60fps animations or complex UIs with hundreds of components - neither applies here.

---

## Q12: Binary Name

**Decision:** `acd` (short) / `agent-console-dashboard` (full)
**Date:** 2026-01-17
**Status:** Resolved

### Names - Binary Name

| Form           | Name                      |
| -------------- | ------------------------- |
| Binary (short) | `acd`                     |
| Binary (full)  | `agent-console-dashboard` |
| Crate name     | `agent-console-dashboard` |

### Usage - Binary Name

```bash
# Short form
acd daemon
acd tui

# Full form (same binary, symlinked or alias)
agent-console-dashboard daemon
```

---

## Q13: Hook Migration

**Decision:** Not applicable
**Date:** 2026-01-17
**Status:** N/A

### Context - Hook Migration

CC-Hub was a quick test, never released. No users to migrate. Start fresh with `agent-console-dashboard`.

---

## Q14: Socket Location

**Decision:** Platform-specific locations
**Date:** 2026-01-17
**Status:** Resolved

### Context - Socket Location

Unix sockets need a file path as an address. The socket file itself stores nothing (0 bytes) - it's just an endpoint for IPC communication.

### What is a Unix Socket File? - Socket Location

```bash
$ ls -l /tmp/acd.sock
srwxr-xr-x  1 pablo  staff  0 Jan 17 12:00 /tmp/acd.sock
#^-- 's' means socket type, size is 0
```

- Not a regular file - just an address
- No disk I/O during communication (all in memory)
- Deleted when daemon stops

### Options Considered - Socket Location

| Location                    | Pros                          | Cons                          |
| --------------------------- | ----------------------------- | ----------------------------- |
| `/tmp/acd.sock`             | Simple                        | Shared by all users           |
| `$XDG_RUNTIME_DIR/acd.sock` | User-specific, Linux standard | macOS doesn't support XDG     |
| `~/.config/.../acd.sock`    | With config                   | Wrong place for runtime files |

### Decision - Socket Location

Use platform-appropriate locations:

| Platform | Socket Location             | Rationale                            |
| -------- | --------------------------- | ------------------------------------ |
| Linux    | `$XDG_RUNTIME_DIR/acd.sock` | XDG standard for runtime files       |
| macOS    | `$TMPDIR/acd.sock`          | Apple's user-specific temp directory |

### Why Not XDG Everywhere? - Socket Location

**XDG = X Desktop Group** (freedesktop.org standard)

macOS does not follow XDG and likely never will. Apple has their own conventions:

| Purpose | Linux (XDG)        | macOS (Apple)            |
| ------- | ------------------ | ------------------------ |
| Config  | `~/.config/`       | `~/Library/Preferences/` |
| Runtime | `$XDG_RUNTIME_DIR` | `$TMPDIR`                |

We use platform-appropriate locations rather than forcing Linux conventions on macOS.

---

## Q15: IPC Protocol Format

**Decision:** JSON (with MessagePack as fallback)
**Date:** 2026-01-17
**Status:** Resolved

### Context - IPC Protocol Format

How should daemon and clients communicate?

### Options Evaluated - IPC Protocol Format

| Format       | Creator              | Year | Speed  | Size        | Readability            |
| ------------ | -------------------- | ---- | ------ | ----------- | ---------------------- |
| JSON         | Douglas Crockford    | 2001 | ~3.5ms | Larger      | Human readable         |
| MessagePack  | Sadayuki Furuhashi   | 2008 | ~1.5ms | 43% smaller | Binary                 |
| Protobuf     | Google               | 2008 | Fast   | Small       | Requires .proto schema |
| RESP (Redis) | Salvatore Sanfilippo | 2009 | Fast   | Medium      | Text-based             |

### Why Not Protobuf or RESP? - IPC Protocol Format

- **Protobuf:** Requires `.proto` schema files and code generation step - overkill for our simple messages
- **RESP:** Designed for key-value store operations - wrong fit for our use case

### Decision Rationale - IPC Protocol Format

**JSON chosen because:**

- Human-readable for easier debugging
- Performance difference negligible at our message frequency (few per minute)
- Same serde structs work with both JSON and MessagePack
- Very mature (24+ years)

**MessagePack as fallback:** If JSON becomes too slow, switch to MessagePack. Serde makes this a minimal code change - just swap the serializer.

### MessagePack Background - IPC Protocol Format

For future reference if we need to switch:

- Creator: Sadayuki Furuhashi (2008, Japanese developer)
- Maintainer: Open source community, msgpack.org
- Used by: Redis, Fluentd, Pinterest, Tarantool
- Rust crate: `rmp-serde` (works with serde)
- Maturity: 16+ years, production-proven

---

## Q16: Session Identification

**Decision:** Use Claude Code's session_id + derive display_name from cwd
**Date:** 2026-01-17
**Status:** Resolved

### Context - Session Identification

How do we identify sessions uniquely while also showing friendly names in the UI?

### Claude Code Hook Data - Session Identification

Claude Code provides via JSON stdin to hooks:

```json
{
  "session_id": "abc123",
  "cwd": "/Users/pablo/projects/my-app",
  "transcript_path": "/Users/.../.claude/projects/.../abc123.jsonl"
}
```

### Decision - Session Identification

| Field          | Source                                   | Example                        |
| -------------- | ---------------------------------------- | ------------------------------ |
| `session_id`   | From Claude Code directly                | `abc123`                       |
| `display_name` | Derived from `cwd` (last path component) | `my-app`                       |
| `cwd`          | From Claude Code directly                | `/Users/pablo/projects/my-app` |

### Message Protocol - Session Identification

**Send full payload every time** (no separate register step):

```json
{
  "session_id": "abc123",
  "display_name": "my-app",
  "cwd": "/Users/pablo/projects/my-app",
  "status": "working"
}
```

### Intentional Redundancy - Session Identification

**For maintainers/programmers:**

The protocol sends `display_name` and `cwd` with every status update, even though they rarely change. This is **intentional redundancy** for simplicity:

- Avoids separate register/update logic
- Handles edge cases (directory changes) automatically
- Daemon is stateless about "what fields were sent before"

**Do not "optimize" this by caching display_name** - the current design is simpler and more robust.

---

## Q17: Session Discovery

**Decision:** Auto-create on first message
**Date:** 2026-01-17
**Status:** Resolved

### Context - Session Discovery

How does a new session register with the daemon?

### Decision - Session Discovery

No explicit registration needed. Daemon auto-creates sessions:

1. Receive message with `session_id`
2. If `session_id` unknown → create new session entry
3. If `session_id` known → update existing session

This follows from Q16's "full payload every time" decision - daemon has all info needed to create a session from any message.

---

## Q18: Session Closed Detection

**Decision:** SessionEnd hook only (no timeout)
**Date:** 2026-01-17
**Status:** Resolved

### Context - Session Closed Detection

How do we know a session ended?

### Options Considered - Session Closed Detection

1. **SessionEnd hook** - Claude Code fires this when session ends
2. **Timeout** - No updates for X minutes → assume closed
3. **Both**

### Decision Rationale - Session Closed Detection

**SessionEnd hook only because:**

- We trust Claude Code's SessionEnd hook to fire on clean exits
- Timeout adds complexity
- Timeout could incorrectly mark active (but idle) sessions as closed

**If Claude Code crashes without firing hook:** Session stays visible. User can manually remove orphaned sessions. This is acceptable.

### Session Lifecycle Workflow - Session Closed Detection

This connects Q4 (statuses), Q5 (TTL), and Q18 (detection):

```text
┌─────────────────────────────────────────────────────────────────┐
│                    SESSION LIFECYCLE                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. Session starts                                              │
│     └─► Status: Working                                         │
│                                                                 │
│  2. Session runs (status changes via hooks)                     │
│     └─► Working ↔ Attention ↔ Question                          │
│                                                                 │
│  3. Session ends (SessionEnd hook fires)                        │
│     └─► Status: Closed                                          │
│         Session remains visible in dashboard                    │
│                                                                 │
│  4. Closed session fate (one of three):                         │
│     ├─► User resurrects → Status back to Working                │
│     ├─► TTL expires → Removed from dashboard automatically      │
│     └─► User manually removes → Removed from dashboard          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Related Decisions - Session Closed Detection

- **Q4:** Closed is one of 4 statuses (Working, Attention, Question, Closed)
- **Q5:** Closed sessions have configurable TTL + manual removal option

---

## Q19: Resurrection Mechanism

**Decision:** Working directory only (session_id unused)
**Date:** 2026-01-17
**Status:** Resolved

### Context - Resurrection Mechanism

When user wants to resurrect a closed session, what information do we need?

### Options Considered - Resurrection Mechanism

1. **Session ID + working directory** - Full info, precise resume
2. **Full command to restart** - Store exact claude invocation
3. **Claude Code's `--resume` flag** - Use native resume with session_id

### Decision Rationale - Resurrection Mechanism

**Working directory only chosen because:**

- Claude Code has its own session picker when multiple sessions exist per directory
- User can select which session to resume from Claude Code's list
- Simpler implementation - no session_id tracking complexity

### Implementation - Resurrection Mechanism

```rust
/// Resurrect a closed session
///
/// Note: session_id is passed but not used in v0/v1.
/// We rely on Claude Code's session picker when multiple
/// sessions exist for the same working directory.
fn resurrect(cwd: &Path, _session_id: &str) {
    // Start Claude Code in cwd, let user pick session
}
```

### Future Consideration (v2+) - Resurrection Mechanism

May use `session_id` with `claude --resume <session_id>` for more elegant solution that bypasses the session picker.

---

## Q20: Dashboard Refresh Rate

**Decision:** Hybrid with low-accuracy tick
**Date:** 2026-01-17
**Status:** Resolved

### Context - Dashboard Refresh Rate

How often does the TUI dashboard update?

### Options Considered - Dashboard Refresh Rate

1. **Every second** - Precise elapsed time display
2. **Event only** - Push from daemon, no polling
3. **Hybrid** - Events + periodic tick for elapsed time

### Decision Rationale - Dashboard Refresh Rate

**Hybrid with low-accuracy tick chosen because:**

- **Event-driven updates:** Daemon pushes status changes (no polling)
- **Local tick:** Dashboard maintains its own timer for elapsed time display
- **Low accuracy acceptable:** 2-5 second tick interval, not precise 1-second
- **Minute-level display:** "5m" vs "5m 3s" - users don't need second precision

### Why Low Accuracy? - Dashboard Refresh Rate

- Saves CPU resources
- Recalculating display every tick is wasteful
- Elapsed time is informational, not critical
- User doesn't notice 2-3 second drift

### Resource Efficiency - Dashboard Refresh Rate

```text
Event-only: Dashboard waits, uses ~0 CPU
Low-accuracy tick: Wake every 2-5 seconds, minimal CPU
Precise 1-second: Wake every second, unnecessary overhead
```

---

## Q21: Click/Selection Detection

**Decision:** Both mouse and keyboard
**Date:** 2026-01-17
**Status:** Resolved

### Context - Click/Selection Detection

How do users interact with the expandable session widget?

### Options Considered - Click/Selection Detection

1. **Mouse only** - Click to select/expand
2. **Keyboard only** - j/k navigation, Enter to select
3. **Both** - Full input support

### Decision Rationale - Click/Selection Detection

**Both chosen because:**

- Terminal users often prefer keyboard (power users)
- Mouse is convenient for quick selection
- Ratatui supports both natively via crossterm
- No additional complexity to support both

### Implementation - Click/Selection Detection

| Input | Action                 |
| ----- | ---------------------- |
| j / ↓ | Move selection down    |
| k / ↑ | Move selection up      |
| Enter | Toggle expand/collapse |
| Click | Select and toggle      |

---

## Q22: Multiple Dashboards

**Decision:** Yes, all receive same updates
**Date:** 2026-01-17
**Status:** Resolved

### Context - Multiple Dashboards

Can multiple dashboard instances connect to the same daemon?

### Options Considered - Multiple Dashboards

1. **Yes, all receive same updates** - Broadcast model
2. **No, only one allowed** - Exclusive connection
3. **Yes, but read-only for extras** - Primary/secondary model

### Decision Rationale - Multiple Dashboards

**Broadcast model chosen because:**

- Simple implementation - daemon broadcasts to all connected clients
- Expected usage: one dashboard per session/pane
- No reason to restrict dashboard count
- All dashboards show identical state (consistent view)

### Architecture Note - Multiple Dashboards

Many dashboards expected. Typical setup:

- Each Claude Code session runs in its own terminal pane
- Each pane has its own dashboard widget attached
- Dashboard count ≥ session count

---

## Q23: Windows Support

**Decision:** Deferred to v2+ (Named Pipes)
**Date:** 2026-01-17
**Status:** Deferred

### Context - Windows Support

Do we support Windows in v0/v1?

### Options Considered - Windows Support

1. **Named Pipes** - Windows equivalent of Unix sockets
2. **TCP localhost** - Cross-platform but more overhead
3. **No Windows** - Unix only (macOS/Linux)

### Decision Rationale - Windows Support

**Deferred to v2+ because:**

- Focus on core features first (macOS/Linux)
- Windows adds platform-specific code paths
- v0/v1 scope already large enough

### Windows IPC for v2+ - Windows Support

**Named Pipes chosen over TCP localhost because:**

- Windows native equivalent of Unix sockets
- Local IPC only (same security model)
- Fast, kernel-level
- Tokio supports it: `tokio::net::windows::named_pipe`

**No fallback** - if Named Pipes don't work stably, the implementation is rejected. Feature is simple enough that fallbacks indicate deeper problems.

### Platform Socket/Pipe Locations - Windows Support

| Platform | IPC Mechanism | Location                    |
| -------- | ------------- | --------------------------- |
| Linux    | Unix socket   | `$XDG_RUNTIME_DIR/acd.sock` |
| macOS    | Unix socket   | `$TMPDIR/acd.sock`          |
| Windows  | Named Pipe    | `\\.\pipe\acd`              |

### Why Not TCP Localhost? - Windows Support

- More overhead than Named Pipes
- Opens network port (even if localhost-only)
- Named Pipes are the Windows-idiomatic solution
- If Named Pipes fail, TCP won't be more reliable

---

## Q24: Daemon Crash Recovery

**Decision:** Basic recovery (options 1+2), auto-restart deferred
**Date:** 2026-01-17
**Status:** Resolved

### Context - Daemon Crash Recovery

What happens if daemon crashes while sessions are running?

### Options Considered - Daemon Crash Recovery

1. **Sessions re-register on next hook event** - Natural recovery
2. **Dashboard shows error indicator** - User awareness
3. **Auto-restart daemon** - Automatic recovery

### Decision Rationale - Daemon Crash Recovery

**Options 1 + 2 for v0/v1 because:**

- Natural recovery works: next hook event auto-starts daemon (Q2)
- Dashboard shows "?" or error indicator for unknown state
- Simple, no coordination needed

**Option 3 (auto-restart) deferred to v2+ because:**

- **Complexity:** Who triggers restart? Which dashboard process?
- **Many dashboards:** One per session expected, coordination is hard
- **Resource overhead:** Watchdog process adds complexity
- **Low value:** Feature is simple, crashes should be rare
- **Edge case:** Hard to test, not in normal flow

### Recovery Flow - Daemon Crash Recovery

```text
1. Daemon crashes
2. Dashboard detects disconnect → shows "?" indicator
3. User continues working (or notices error)
4. Next hook fires → daemon auto-starts (Q2)
5. Sessions re-register via hooks
6. Dashboard reconnects → normal display resumes
```

### Why No Watchdog? - Daemon Crash Recovery

Auto-restart requires a watchdog process. Questions:

- Which process runs the watchdog?
- If dashboard runs it, which dashboard (many exist)?
- If external, adds deployment complexity

For v0/v1, natural recovery through hooks is sufficient.

---

## Q25: Daemon Shutdown

**Decision:** Multiple mechanisms (stop command, SIGTERM, auto-stop)
**Date:** 2026-01-17
**Status:** Resolved

### Context - Daemon Shutdown

How do users stop the daemon gracefully?

### Options Considered - Daemon Shutdown

1. **CLI command** - `acd stop`
2. **Signal** - SIGTERM
3. **Socket command** - Send "SHUTDOWN" message
4. **Auto-stop** - When no clients connected for X time

### Decision - Daemon Shutdown

Use multiple mechanisms:

| Mechanism                 | Behavior                                  |
| ------------------------- | ----------------------------------------- |
| `acd stop`                | Warns if dashboards connected, then stops |
| `acd stop --force` / `-f` | Stops immediately, no warning             |
| SIGTERM                   | Same as `acd stop` (graceful shutdown)    |
| Auto-stop                 | After configurable idle time              |

### Dashboard Tracking - Daemon Shutdown

| Version | What we track             | How             |
| ------- | ------------------------- | --------------- |
| v0/v1   | Connection count only     | `clients.len()` |
| v2      | PIDs via peer credentials | `nix` crate     |

**Why defer PID tracking:** v0/v1 only needs count for stop warning. PID tracking adds dependency (`nix` crate) for minimal benefit.

### Auto-Stop Implementation - Daemon Shutdown

```rust
/// Configuration constants (not magic values)
const AUTO_STOP_CHECK_INTERVAL_SECS: u64 = 300;   // 5 minutes
const AUTO_STOP_IDLE_THRESHOLD_SECS: u64 = 1800;  // 30 minutes
```

Auto-stop triggers when:

1. No dashboards connected, AND
2. No active sessions, AND
3. Condition persists for `IDLE_THRESHOLD` duration

**Resource usage:** Near zero. Process sleeps (kernel timer), wakes every 5 minutes to check condition (~1ms of CPU).

### SIGTERM Handling - Daemon Shutdown

Daemon registers signal handler via Tokio:

```rust
use tokio::signal::unix::{signal, SignalKind};

let mut sigterm = signal(SignalKind::terminate())?;

tokio::select! {
    _ = daemon_main_loop() => {}
    _ = sigterm.recv() => {
        graceful_shutdown().await;
    }
}
```

### v2 Enhancements - Daemon Shutdown

| Feature              | Implementation                           | Binary size impact |
| -------------------- | ---------------------------------------- | ------------------ |
| Desktop notification | Shell out to `notify-send` / `osascript` | 0 bytes            |
| PID tracking         | `nix` crate                              | ~50-100 KB         |
| Log message          | Built-in logging                         | 0 bytes            |

### Why Not Socket Command? - Daemon Shutdown

Socket command ("SHUTDOWN") would require:

- Dashboard or CLI to connect and send command
- Same as `acd stop` but less discoverable

`acd stop` is clearer UX.

---

## Q26: Signal Handling

**Decision:** SIGTERM/SIGINT = shutdown, SIGHUP = reload config
**Date:** 2026-01-17
**Status:** Resolved

### Context - Signal Handling

How should daemon respond to Unix signals?

### Decision - Signal Handling

| Signal  | Behavior                                    |
| ------- | ------------------------------------------- |
| SIGTERM | Graceful shutdown (same as `acd stop`)      |
| SIGINT  | Same as SIGTERM (for foreground/debug mode) |
| SIGHUP  | Reload configuration (see Q27)              |

### Signal Reference - Signal Handling

| Signal  | Number | Origin                          | Can be caught? |
| ------- | ------ | ------------------------------- | -------------- |
| SIGINT  | 2      | Ctrl+C                          | Yes            |
| SIGHUP  | 1      | "Hang Up" (terminal disconnect) | Yes            |
| SIGTERM | 15     | Terminate request               | Yes            |
| SIGKILL | 9      | Force kill                      | No             |

### Implementation - Signal Handling

```rust
use tokio::signal::unix::{signal, SignalKind};

let mut sigterm = signal(SignalKind::terminate())?;
let mut sigint = signal(SignalKind::interrupt())?;
let mut sighup = signal(SignalKind::hangup())?;

tokio::select! {
    _ = sigterm.recv() => graceful_shutdown().await,
    _ = sigint.recv() => graceful_shutdown().await,
    _ = sighup.recv() => reload_config().await,
}
```

---

## Q27: Config Reload

**Decision:** Hot reload in v0
**Date:** 2026-01-17
**Status:** Resolved

### Context - Config Reload

Can configuration be changed without restarting daemon?

### Decision - Config Reload

**Hot reload supported in v0** - helps development significantly.

### Trigger Methods - Config Reload

| Method | Command           |
| ------ | ----------------- |
| Signal | `kill -HUP <pid>` |
| CLI    | `acd reload`      |

### What Can Be Hot-Reloaded - Config Reload

| Setting              | Hot-reloadable?           |
| -------------------- | ------------------------- |
| Colors               | Yes                       |
| Tick interval        | Yes                       |
| Display mode         | Yes                       |
| Auto-stop thresholds | Yes                       |
| Socket path          | **No** (restart required) |
| Log file location    | **No** (restart required) |

### Invalid Config Handling - Config Reload

- Keep old config if new config is invalid
- Log error: "Config reload failed: invalid value for X"
- Daemon continues running with previous config

### Why Hot Reload in v0? - Config Reload

- Speeds up development iteration
- Test UI changes without restart
- Standard daemon behavior (matches user expectations)

---

## Q28: Startup State

**Decision:** Message with README reference
**Date:** 2026-01-17
**Status:** Resolved

### Context - Startup State

What does dashboard show when no sessions are running?

### Options Considered - Startup State

1. **Simple message:** "No active sessions"
2. **Instructions:** "Start Claude Code to see sessions here"
3. **Blank:** Nothing
4. **ASCII art / logo**

### Decision - Startup State

**Message with README reference:**

```text
No active sessions. See README for setup.
```

### Rationale - Startup State

- Project may extend beyond Claude Code
- Could become general notification center for Zellij/other multiplexers
- README is the right place for setup instructions
- Avoids hardcoding specific tool names in UI
