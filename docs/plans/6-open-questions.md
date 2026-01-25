# Open Questions

Unresolved decisions requiring discussion or research.

---

## Architecture

### Q1: Backend Architecture

**Question:** Which backend approach should we use?

| Option        | Pros                         | Cons                       |
| ------------- | ---------------------------- | -------------------------- |
| Single Daemon | Clean, shared state          | Lifecycle management       |
| Shared Memory | No daemon                    | Complex, platform-specific |
| SQLite        | Robust, optional persistence | Disk I/O                   |

**Current Leaning:** Single Daemon (matches PRD)

**Decision Needed By:** Before implementation starts

---

### Q2: Daemon Auto-Start

**Question:** Should daemon auto-start if not running when client connects?

**Options:**

- Yes: Client starts daemon if socket doesn't exist
- No: Require explicit daemon start (systemd/launchd)
- Hybrid: Auto-start in foreground, manual for background

**Considerations:**

- Auto-start is convenient but hides startup errors
- Manual start is explicit but requires setup

---

### Q3: Config File Location

**Question:** Where should config file live?

**Options:**

- `~/.config/agent-console/config.toml` (XDG)
- `~/.agent-console.toml` (home directory)
- `~/.agent-console/config.toml` (custom directory)

**Current Leaning:** XDG standard (`~/.config/agent-console/`)

---

## Features

### Q4: Additional Status Types

**Question:** Should we support more status types beyond
working/attention/question?

**Current statuses:**

- Working
- Attention
- Question
- Closed

**Potential additions:**

- Error (agent crashed)
- Paused (user paused session)
- Rate Limited (API limit hit)

**Consideration:** Keep simple vs. provide granularity

---

### Q5: Session Resurrection Scope

**Question:** How long do we keep closed sessions for resurrection?

**Options:**

- Until daemon restart (ephemeral)
- Configurable time limit (e.g., 24 hours)
- Forever until manually removed

**Consideration:** Memory usage vs. convenience

**Decision (revised):** Dedicated history space, no TTL

- Closed sessions move to history (not shown in main dashboard)
- Persisted to file: `~/.config/agent-console/history.json`
- No automatic removal - user manages manually
- Main dashboard stays clean (active sessions only)

**Access methods:**

- TUI keyboard shortcut (e.g., `h` for history)
- CLI command: `acd history`

**History view shows:**

- Closed sessions (can resurrect or remove)
- Past state transitions per session
- Session metadata (working dir, timestamps)

---

### Q6: API Usage Source

**Question:** How do we get API usage data?

**Options:**

- Hook-provided (if Claude Code exposes it)
- Log parsing (fragile)
- Estimate from message count
- Direct Anthropic API query (requires key)

**Investigation Needed:** What does Claude Code expose?

---

## Integrations

### Q7: AskQuestion Hook

**Question:** Does Claude Code have an AskQuestion hook?

**Status:** Unknown, needs investigation

**If No:** Should we request this feature from Anthropic?

---

### Q8: Zellij Plugin

**Question:** Should we build a native Zellij plugin?

**Pros:**

- Deeper integration
- No hooks needed
- Native status indicators

**Cons:**

- WASM complexity
- Maintenance burden
- Zellij-specific

**Current Leaning:** Not for v1, evaluate later

---

### Q9: Tmux Plugin

**Question:** Should we build a Tmux plugin?

**Same considerations as Zellij plugin.**

**Current Leaning:** Not for v1

---

## UI

### Q10: Default Layout

**Question:** What should the default layout be?

**Options:**

- `one-line` (v1 compatible, minimal)
- `two-line` (adds working directory)
- `detailed` (full info)

**Current Leaning:** `two-line` as default, `one-line` available

---

### Q11: TUI Framework

**Question:** Ratatui is confirmed, but which component style?

**Options:**

- Immediate mode (redraw everything each frame)
- Retained mode (component state)

**Current Leaning:** Immediate mode (simpler, standard for Ratatui)

---

## Implementation

### Q12: Project Name / Binary Name

**Question:** What should the binary be called?

**Options:**

- `agent-console` (matches project name)
- `ac` (short alias)
- `acd` (agent console dashboard)

**Current Leaning:** `agent-console` with `ac` as optional alias

---

### Q13: Hook Migration

**Question:** How do we migrate from cc-hub hooks to agent-console hooks?

**Options:**

- Update hooks in place
- Provide migration script
- Support both during transition

---

## Implementation (Phase 2)

### Q14: Socket Location

**Question:** Where does the daemon socket live?

**Options:**

- `/tmp/acd.sock` (simple, cleared on reboot)
- `$XDG_RUNTIME_DIR/acd.sock` (XDG standard, user-specific)
- `~/.config/agent-console-dashboard/acd.sock` (with config)

**Context:** Unix socket is just an address for IPC, not a real file. No
content, no disk I/O during communication.

**Decision:** Platform-specific locations

| Platform | Socket Location             |
| -------- | --------------------------- |
| Linux    | `$XDG_RUNTIME_DIR/acd.sock` |
| macOS    | `$TMPDIR/acd.sock`          |

**Rationale:** Each platform has its own standard for runtime files. macOS
doesn't support XDG.

---

### Q15: IPC Protocol Format

**Question:** What format for daemon communication?

**Options:**

| Format      | Speed  | Size        | Readability    |
| ----------- | ------ | ----------- | -------------- |
| JSON        | ~3.5ms | Larger      | Human readable |
| MessagePack | ~1.5ms | 43% smaller | Binary         |
| Custom text | Varies | Medium      | Human readable |

**Decision:** JSON

**Rationale:**

- Human-readable for easier debugging
- Performance difference negligible at our message frequency
- Same serde structs work with both formats

**Alternative considered:** MessagePack (faster, 43% smaller). If JSON becomes
too slow, switch to MessagePack - serde makes this a minimal code change.

---

### Q16: Session Identification

**Question:** How do we identify sessions?

**Fields needed:**

| Field          | Purpose           | Example      |
| -------------- | ----------------- | ------------ |
| `session_id`   | Unique identifier | UUID or hash |
| `display_name` | Shown in UI       | `my-project` |

**Options for ID generation:**

- UUID (guaranteed unique)
- Hash of: working_dir + start_time + PID
- From Claude Code if exposed in hooks

**Decision:** Use Claude Code's `session_id`, derive `display_name` from `cwd`

**Message format:** Send full payload every time (no separate register step)

```json
{
  "session_id": "abc123",
  "display_name": "my-app",
  "cwd": "/Users/pablo/projects/my-app",
  "status": "working"
}
```

**Rationale:** Redundancy is acceptable. Simpler than maintaining registration
state. Handles edge cases like directory changes automatically.

---

### Q17: Session Discovery

**Question:** How does a new session register with daemon?

**Options:**

- Hook sends "register" on SessionStart
- Hook sends status update, daemon auto-creates session
- Both

**Decision:** Auto-create on first message

Daemon behavior:

1. Receive message with `session_id`
2. If `session_id` unknown → create new session entry
3. If `session_id` known → update existing session

No separate registration step required.

---

### Implementation Note: Intentional Redundancy

**For maintainers/programmers:**

The message protocol sends `display_name` and `cwd` with every status update,
even though they rarely change. This is **intentional redundancy** for
simplicity:

- Avoids separate register/update logic
- Handles edge cases (directory changes) automatically
- Daemon is stateless about "what fields were sent before"

Do not "optimize" this by caching display_name - the current design is simpler
and more robust.

---

### Q18: Session Closed Detection

**Question:** How do we know a session ended?

**Options:**

- SessionEnd hook from Claude Code
- Timeout (no updates for X minutes)
- Both

**Decision:** SessionEnd hook only (no timeout)

### Session Lifecycle Workflow

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

### Why No Timeout?

- We trust Claude Code's SessionEnd hook to fire on clean exits
- If Claude Code crashes without firing hook, the session stays visible
- This is acceptable: user can manually remove orphaned sessions
- Timeout would add complexity and could incorrectly mark active sessions as
  closed

### Related Decisions

- **Q4:** Closed is one of 4 statuses (Working, Attention, Question, Closed)
- **Q5:** Closed sessions have configurable TTL + manual removal option

---

### Q19: Resurrection Mechanism

**Question:** What state do we save for resurrection?

**Options:**

- Just session ID + working directory
- Full command to restart Claude Code
- Use Claude Code's `--resume` flag with session ID

**Decision:** Working directory only

- `session_id` passed to function but marked as unused
- Claude Code has its own session picker when multiple sessions exist per
  directory
- v2+ may use `session_id` for more elegant solution

---

### Q20: Dashboard Refresh Rate

**Question:** How often does TUI update?

**Options:**

- Every second (for elapsed time display)
- On event only (push from daemon)
- Hybrid (events + 1s tick for clock)

**Decision:** Hybrid with low-accuracy tick

- Event-driven: Daemon pushes status changes
- Local tick: 2-5 seconds for elapsed time display (not precise)
- Display: Minute-level granularity sufficient
- Rationale: Saves resources, accuracy not important for elapsed time

---

### Q21: Click/Selection Detection

**Question:** For expandable widget, how do we detect selection?

**Options:**

- Mouse events in Ratatui
- Keyboard only (j/k + Enter)
- Both

**Decision:** Both mouse and keyboard

- Keyboard: j/k or arrow keys for navigation, Enter to select
- Mouse: Click to select/expand

---

### Q22: Multiple Dashboards

**Question:** Can multiple dashboard instances connect to same daemon?

**Options:**

- Yes, all receive same updates
- No, only one allowed
- Yes, but read-only for additional instances

**Decision:** Yes, all receive same updates

- Daemon broadcasts to all connected clients
- No restrictions on instance count
- Expected usage: one dashboard per session/pane

---

### Q23: Windows Support

**Question:** Do we support Windows?

**Options:**

- Yes (use Named Pipes instead of Unix socket)
- Yes (use TCP localhost)
- No, Unix only (macOS/Linux)
- Later (v2+)

**Decision:** Deferred to v2+

- v0/v1: Unix only (macOS/Linux)
- v2+: Consider Windows support using Named Pipes

**Windows IPC (for v2+ reference):**

| Platform | IPC Mechanism                            |
| -------- | ---------------------------------------- |
| Linux    | Unix socket: `$XDG_RUNTIME_DIR/acd.sock` |
| macOS    | Unix socket: `$TMPDIR/acd.sock`          |
| Windows  | Named Pipe: `\\.\pipe\acd`               |

**Why Named Pipes (not TCP localhost):**

- Windows equivalent of Unix sockets
- Local IPC only (like Unix sockets)
- Fast, kernel-level
- Tokio supports it: `tokio::net::windows::named_pipe`
- No fallback - if implementation doesn't work stably, it's not a good option

---

### Q24: Daemon Crash Recovery

**Question:** What if daemon crashes while sessions are running?

**Options:**

- Sessions re-register on next hook event
- Dashboard shows "daemon offline" message
- Auto-restart daemon

**Decision:** Basic recovery in v0/v1

- **Options 1 + 2 only:**
  - Hooks re-register on next event (daemon auto-starts per Q2)
  - Dashboard shows "?" or error indicator for unknown state
- **Option 3 (auto-restart) deferred to v2+:**
  - Complex: who triggers restart? Which dashboard?
  - Many dashboards expected (one per session)
  - Coordination overhead exceeds benefit
  - Feature is simple enough it shouldn't fail often

---

## Daemon Lifecycle (Q25-Q28)

### Q25: Daemon Shutdown

**Question:** How do we stop the daemon gracefully?

**Options:**

- CLI command: `acd stop`
- Signal: SIGTERM
- Socket command: send "SHUTDOWN" message
- Auto-stop when no clients connected

**Sub-questions:**

- Does daemon track connected dashboards?
- Should daemon refuse to stop if dashboards are connected?
- Or warn user and force stop?

**Decision:** Multiple mechanisms

| Mechanism                 | Behavior                                                |
| ------------------------- | ------------------------------------------------------- |
| `acd stop`                | Warns if dashboards connected, stops                    |
| `acd stop --force` / `-f` | Stops immediately, no warning                           |
| SIGTERM                   | Same as `acd stop` (graceful shutdown)                  |
| Auto-stop                 | After configurable idle time (no clients + no sessions) |

**Dashboard tracking:**

- v0/v1: Count connections only (no PID tracking)
- v2: Track PIDs via `nix` crate for detailed info

**v2 enhancements:**

- Desktop notification when last dashboard disconnects (shell out, 0 binary
  increase)
- Log message: "Dashboard disconnected"

**Constants (not magic values):**

```rust
const AUTO_STOP_CHECK_INTERVAL_SECS: u64 = 300;   // 5 minutes
const AUTO_STOP_IDLE_THRESHOLD_SECS: u64 = 1800;  // 30 minutes
```

---

### Q26: Signal Handling

**Question:** How does daemon respond to signals?

| Signal  | Options                |
| ------- | ---------------------- |
| SIGTERM | Graceful shutdown      |
| SIGINT  | Same as SIGTERM?       |
| SIGHUP  | Reload config? Ignore? |

**Decision:**

| Signal  | Behavior                                    |
| ------- | ------------------------------------------- |
| SIGTERM | Graceful shutdown (same as `acd stop`)      |
| SIGINT  | Same as SIGTERM (for foreground/debug mode) |
| SIGHUP  | Reload config (see Q27)                     |

**SIGHUP = Signal Hang Up:** Originally meant terminal disconnected. Modern
convention: tell daemons to reload config.

---

### Q27: Config Reload

**Question:** Can config be reloaded without restart?

**Options:**

- Hot reload (SIGHUP or command)
- Restart required
- Some settings hot-reloadable, others require restart

**Consideration:** What if new config is invalid?

**Decision:** Hot reload in v0

**Trigger methods:**

- SIGHUP signal: `kill -HUP <pid>`
- CLI command: `acd reload`

**Hot-reloadable settings:**

- Colors
- Tick interval
- Display mode (compact/full)
- Auto-stop thresholds

**Requires restart:**

- Socket path (already listening on old path)
- Log file location

**Invalid config handling:**

- Keep old config if new config is invalid
- Log error: "Config reload failed: invalid value for X"

**Rationale:** Hot reload helps development process significantly.

---

### Q28: Startup State

**Question:** What does dashboard show when no sessions exist?

**Options:**

- Empty state message: "No active sessions"
- Instructions: "Start Claude Code to see sessions here"
- Nothing (blank area)

**Decision:** Message with README reference

```text
No active sessions. See README for setup.
```

**Rationale:** Project may extend beyond Claude Code to general notification
center for Zellij/multiplexers. Point to README instead of specific tool
instructions.

---

## Error Handling (Q29-Q32)

### Q29: Hook Timeout

**Question:** What if hook command hangs?

**Options:**

- No timeout (trust hooks)
- Configurable timeout (e.g., 5 seconds)
- Kill hung hooks after timeout

**Consideration:** Hook is external process, we can't control it.

**Decision:** 5s timeout, exit 1 on failure

| Scenario           | Behavior                           |
| ------------------ | ---------------------------------- |
| Daemon unreachable | `acd set` exits 1 after 5s timeout |
| Socket write hangs | `acd set` exits 1 after 5s timeout |
| Daemon responds    | `acd set` exits 0 immediately      |

**Exit codes (per Q50):**

- Exit 0: Success - Claude continues normally
- Exit 1: Non-blocking failure - warning shown, Claude continues
- Never exit 2 (would block Claude from proceeding)

---

### Q50: Claude Code Hook Contract

**Question:** What does Claude Code expect from hooks?

**Decision:** Contract documented below

**Exit codes:**

| Exit Code | Behavior           | stdout                         | stderr                        |
| --------- | ------------------ | ------------------------------ | ----------------------------- |
| 0         | Success            | Shown in verbose mode (Ctrl+O) | Ignored                       |
| 2         | Blocking error     | Ignored                        | Error message shown to Claude |
| Other     | Non-blocking error | Ignored                        | Shown with warning prefix     |

**Timeout:** 60s default, configurable per hook via `timeout` field (in seconds)

**Blocking:** All hooks are blocking (Claude waits), but multiple hooks run in
parallel

**Hook input:** JSON via stdin with `session_id`, `cwd`, `hook_event_name`, etc.

**Available hooks:** SessionStart, SessionEnd, PreToolUse, PostToolUse,
UserPromptSubmit, Stop, and more (12 total)

**Hook configuration for Claude Code:**

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "acd set --claude-hook PreToolUse",
            "timeout": 5
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "acd set --claude-hook Stop",
            "timeout": 5
          }
        ]
      }
    ],
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "acd set --claude-hook SessionStart",
            "timeout": 5
          }
        ]
      }
    ],
    "SessionEnd": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "acd set --claude-hook SessionEnd",
            "timeout": 5
          }
        ]
      }
    ]
  }
}
```

**`acd set --claude-hook` behavior:**

| Flag Value   | Status Mapping |
| ------------ | -------------- |
| PreToolUse   | working        |
| SessionStart | working        |
| Stop         | attention      |
| SessionEnd   | closed         |

**Exit codes:**

- Exit 0: Success (daemon reached)
- Exit 1: Non-blocking failure (daemon unreachable)
- Never exit 2 (would block Claude)

**Error handling:** On failure, `acd set` broadcasts error to all connected
dashboards (not to Claude - wastes context space). Dashboard shows "Hook error:
daemon unreachable" or similar.

**Timeout:** 5s recommended (configurable in Claude Code's hook config, not by
us).

**Updates Q29:** 5s timeout appropriate. If daemon unreachable, exit 1,
broadcast error to dashboards.

---

### Q51: Per-Status Duration Tracking

**Question:** Should we track time spent in each status?

**Current state:** We track time waiting for attention. But not time spent
working.

**Decision:** Track duration for all statuses

- Track timestamps for each status: `working_started_at`,
  `attention_started_at`, etc.
- Can show: "worked 45min, waited 5min"
- Data structure changes needed (implementation detail, defer)

---

### Q52: CWD vs Session Directory

**Question:** What if Claude Code changes directory mid-session?

**Problem:** If we use `cwd` from each hook event, display_name might change
when user does `cd /other/path`.

**Decision:** Always show session directory

- Display name = session directory (not current cwd)
- If session directory unknown, use first cwd as session directory
- Implementation: check if Claude Code provides session_dir, otherwise use first
  cwd
- Display name stays stable throughout session

---

### Q30: Invalid Messages

**Question:** What if daemon receives malformed JSON?

**Options:**

- Log warning, ignore message
- Return error to client
- Disconnect client

**Decision:** Return error to client

- Send error response: `{"error": "invalid JSON: <parse error>"}`
- Log warning on daemon side
- Keep connection open for retry
- Matches request/response semantics

---

### Q31: Connection Errors

**Question:** What if dashboard can't connect to daemon?

**Options:**

- Start daemon automatically (Q2: already decided yes)
- Retry with backoff
- Show error and exit

**Sub-question:** How many retries? What backoff strategy?

**Decision:** Auto-start + wait for socket + retry

1. Try connect to daemon
2. If fails, auto-start daemon (Q2)
3. Poll for socket file (up to 2s, check every 100ms)
4. Once socket exists, retry connect (3 attempts, 100ms apart)
5. If still fails, show error and exit

```rust
const SOCKET_POLL_TIMEOUT_MS: u64 = 2000;
const SOCKET_POLL_INTERVAL_MS: u64 = 100;
const CONNECTION_RETRIES: u32 = 3;
const CONNECTION_RETRY_DELAY_MS: u64 = 100;
```

**Rationale:** Separates daemon startup wait from connection retry. Polling for
socket file is more reliable than guessing startup time.

---

### Q32: API Errors

**Question:** What if Anthropic Usage API fails?

**Options:**

- Show stale data with "last updated X ago"
- Show error indicator
- Retry in background
- Hide usage widget

**Decision:** Stale data + background retry + click to refresh

- Show last known data with age: `$1.42 (5m ago)`
- Retry in background every 60s
- After 5 min of failures, show warning: `$1.42 (stale ⚠)`
- Click/select warning to trigger immediate retry

```rust
const API_RETRY_INTERVAL_SECS: u64 = 60;
const API_STALE_WARNING_SECS: u64 = 300;  // 5 minutes
```

**Interaction:** Warning indicator is clickable to force refresh.

---

## Security (Q33-Q35)

### Q33: Socket Permissions

**Question:** Who can connect to daemon socket?

**Options:**

- User-only: `0600` (only socket owner)
- User + group: `0660`
- World-readable: `0666` (not recommended)

**Decision:** User-only (`0600`)

- Only socket owner can connect
- Single-user tool, no need for shared access
- Prevents other users from seeing session data

---

### Q34: API Key Storage

**Question:** Where to store Anthropic API key for usage data?

**Decision:** Reuse Claude Code's OAuth credentials (no separate key needed)

**Implementation:** Completed in
[E011 - Claude Usage Crate](../epic/E011-claude-usage-crate.md). The
`claude-usage` crate (published to crates.io) handles all credential retrieval
and API calls. Use `claude_usage::get_usage()` to fetch account quota data.

**Verified API:**

```text
GET https://api.anthropic.com/api/oauth/usage
Authorization: Bearer <token>
anthropic-beta: oauth-2025-04-20
```

**Token source (macOS):**

```bash
security find-generic-password -s "Claude Code-credentials" -w
# Returns JSON: { "claudeAiOauth": { "accessToken": "sk-ant-oat01-..." } }
```

**Response:**

```json
{
  "five_hour": { "utilization": 8.0, "resets_at": "2026-01-22T09:00:00Z" },
  "seven_day": { "utilization": 77.0, "resets_at": "2026-01-22T19:00:00Z" },
  "seven_day_sonnet": { "utilization": 0.0, "resets_at": "..." },
  "extra_usage": { "is_enabled": false, ... }
}
```

**Fields:**

- `utilization`: Percentage used (0-100)
- `resets_at`: ISO 8601 timestamp

**Platform support:**

| Platform | Credential Location                   | Access Method            |
| -------- | ------------------------------------- | ------------------------ |
| macOS    | Keychain: `"Claude Code-credentials"` | security-framework crate |
| Linux    | `~/.claude/.credentials.json`         | File read + JSON parse   |

**Error handling:**

- Token expired: Credentials include `expiresAt` timestamp, check before use
- Token invalid: API returns 401, show "Re-login to Claude Code" message
- No credentials: Show "Claude Code not logged in" message

**Security requirements:**

- Isolate credential handling to single module (`src/credentials.rs` or similar)
- Read token → make API call → discard token immediately
- Never store token in memory longer than needed
- Never pass token to other modules (only pass the API response data)
- Never log or serialize the token

---

### Q35: Input Validation

**Question:** Do we validate/sanitize inputs?

**Areas:**

- Session names (display_name)
- File paths (cwd)
- Session IDs

**Consideration:** What characters are allowed? Max length?

**Decision:** Basic length limits only

| Field          | Max length | Validation         |
| -------------- | ---------- | ------------------ |
| `display_name` | 64 chars   | Truncate if longer |
| `cwd`          | 256 chars  | Truncate if longer |
| `session_id`   | 128 chars  | Truncate if longer |

- No character restrictions (paths can contain almost anything)
- Input comes from Claude Code (trusted source)
- Just prevent memory issues from absurdly long strings

---

## Limits & Defaults (Q36-Q39)

### Q36: Maximum Sessions

**Question:** Is there a limit on tracked sessions?

**Options:**

- No limit (trust user)
- Soft limit with warning
- Hard limit (reject new sessions)

**Consideration:** Memory usage per session is small (~1KB).

**Decision:** Soft limit with warning

- Warning at 50 sessions: log warning, continue accepting
- No hard limit
- Memory per session ~1KB, not a real concern

```rust
const SESSION_SOFT_LIMIT: usize = 50;
```

---

### Q37: Maximum Dashboards

**Question:** Is there a limit on connected dashboards?

**Options:**

- No limit
- Soft limit with warning
- Hard limit

**Consideration:** Each dashboard is a socket connection.

**Decision:** Soft limit with warning at 50

- Warning at 50 dashboards: log warning, continue accepting
- Same as session limit (Q36) - expect ~1 dashboard per session
- No hard limit

```rust
const DASHBOARD_SOFT_LIMIT: usize = 50;
```

---

### Q38: History Depth

**Question:** How many state transitions to keep per session?

**Options:**

- Unlimited (until session closes)
- Fixed limit (e.g., 100 transitions)
- Time-based (last 24 hours)
- Configurable

**Decision:** Combined limit - last 200 AND within 24h

- Keep max 200 transitions per session
- Drop transitions older than 24 hours
- Whichever limit hits first

```rust
const HISTORY_MAX_ENTRIES: usize = 200;
const HISTORY_MAX_AGE_HOURS: u64 = 24;
```

---

### Q39: Default Configuration

**Question:** What are sensible defaults for all config options?

**Decision:** Defaults documented

| Setting                   | Default                       | Notes                        |
| ------------------------- | ----------------------------- | ---------------------------- |
| `socket_path`             | Platform-specific             | Q14: Linux XDG, macOS TMPDIR |
| `tick_interval_secs`      | 1                             | Configurable                 |
| `layout`                  | 3-line (sessions + API usage) | 2 widgets, expandable        |
| `orientation`             | horizontal                    | Sessions inline with `\|`    |
| `auto_stop_idle_secs`     | 1800                          | Q25: 30 minutes              |
| `client_timeout_secs`     | 5                             | Q29                          |
| `api_retry_interval_secs` | 60                            | Q32                          |
| `history_max_entries`     | 200                           | Q38                          |
| `history_max_age_hours`   | 24                            | Q38                          |
| `session_soft_limit`      | 50                            | Q36                          |
| `dashboard_soft_limit`    | 50                            | Q37                          |

**Note:** `resurrection_ttl` removed - using dedicated history space instead (Q5
revised)

---

## Display & UX (Q40-Q43)

### Q40: Empty State

**Status:** Duplicate of Q28

See Q28 (Startup State) - same question, already resolved.

---

### Q41: Session Name Conflicts

**Question:** What if two sessions have same display_name?

**Scenario:** Two Claude Code sessions in same directory, or directories with
same basename.

**Options:**

- Append number: `my-app`, `my-app (2)`
- Show parent directory: `projects/my-app`, `work/my-app`
- Use session_id suffix: `my-app [abc1]`

**Decision:** Depends on conflict type

| Conflict                      | Resolution                                          |
| ----------------------------- | --------------------------------------------------- |
| Same basename, different path | Show parent: `work/my-app`, `personal/my-app`       |
| Same path (multiple sessions) | Session ID suffix: `my-app [abc1]`, `my-app [def2]` |

---

### Q42: Long Names/Paths

**Question:** How to handle long project names or paths?

**Decision:** Systematic cascade approach

```text
Step 1: Base name
  └─ Use last folder: /a/b/c/my-project → "my-project"

Step 2a: Conflict resolution (only if needed)
  └─ Add parent folders until unique

Step 2b: Length check (if > max_width after Step 2a)
  └─ Keep distinguishing parent(s) FULL
  └─ Abbreviate non-distinguishing parents to first char
  └─ If still too long, truncate middle of final name

Step 3: Final truncation (if still > max_width)
  └─ Truncate middle: "manifold-ag...dashboard"
```

**Example with conflict:**

- `/base/projects/work/my-app` vs `/base-old/projects/work/my-app`
- Distinguishing parent: `base` vs `base-old` (keep full)
- Result: `base/p/w/my-app`, `base-old/p/w/my-app`

**Never abbreviate the distinguishing parent** - that's how users tell them
apart.

**Configuration:**

```toml
[display]
max_name_width = 32  # default
```

**Deferred to v1+:**

- Custom rename feature

---

### Q43: Unicode Support

**Question:** Support non-ASCII characters?

**Areas:**

- Project names with accents: `café-app`
- Emoji in directory names: `🚀-project`
- CJK characters: `项目`

**Consideration:** Terminal font support varies.

**Decision:** Full unicode support

- Display as-is, trust terminal to render
- Input comes from filesystem (already valid)
- Ratatui handles unicode well
- If terminal can't render, that's user's terminal config issue

---

## Distribution (Q44-Q46)

### Q44: Installation Method

**Question:** How do users install?

**Options:**

- `cargo install agent-console-dashboard`
- Homebrew: `brew install acd`
- Binary releases (GitHub Releases)
- Package managers (apt, dnf, pacman)

**Sub-question:** Which platforms for binary releases?

**Decision:** Phased rollout

| Version | Methods                      |
| ------- | ---------------------------- |
| v0      | Cargo + Binary releases      |
| v1+     | Add Homebrew                 |
| Later   | Package managers (on demand) |

**Binary release platforms (v0):**

- macOS (arm64, x86_64)
- Linux (x86_64, arm64)

---

### Q45: Shell Completions

**Question:** Generate shell completions?

**Options:**

- Built-in: `acd completions bash > ~/.bash_completion.d/acd`
- Separate package
- Not for v0/v1

**Shells:** bash, zsh, fish, PowerShell

**Decision:** Built-in via clap_complete

```bash
acd completions bash > ~/.bash_completion.d/acd
acd completions zsh > ~/.zfunc/_acd
acd completions fish > ~/.config/fish/completions/acd.fish
```

Trivial to implement with `clap_complete` crate.

---

### Q46: Man Pages

**Question:** Generate man pages?

**Options:**

- Yes, via clap_mangen
- No, rely on `--help`
- Generate but don't install by default

**Decision:** Defer to v1+

- v0: Rely on `--help` only
- v1+: Consider `clap_mangen` for man pages

---

## Development (Q47-Q49)

### Q47: Logging Strategy

**Question:** Do we log? Where? What format?

**Options:**

- No logging (simple)
- stderr only
- File logging (`~/.local/share/agent-console/logs/`)
- Structured logging (JSON)

**Sub-question:** Log levels? Configurable verbosity?

**Decision:** File logging

- Location: `~/.local/share/agent-console/logs/`
- Format: Plain text (not JSON)
- Levels: error, warn, info, debug
- Configurable via `RUST_LOG` env var or config file
- Rotation: Consider log rotation for long-running daemon

---

### Q48: Testing Strategy

**Question:** How do we test?

**Areas:**

- Unit tests (data structures, parsing)
- Integration tests (daemon + client)
- End-to-end tests (with mock hooks)

**Sub-question:** How to test daemon without real Claude Code?

**Decision:** All three levels

| Level       | What                                    | How                        |
| ----------- | --------------------------------------- | -------------------------- |
| Unit        | Data structures, parsing, display logic | Standard Rust tests        |
| Integration | Daemon + client communication           | Spawn daemon, mock clients |
| End-to-end  | Full workflow with mock hooks           | `acd test-client` command  |

**Mock testing:**

- `acd test-client` command for manual testing
- Sends fake hook events to daemon
- Useful for development and demos

---

### Q49: Feature Flags

**Question:** Use Cargo features for optional functionality?

**Potential features:**

- `clipboard` - copy session info
- `notifications` - desktop notifications
- `tui` - TUI dashboard (maybe always included)

**Decision:** No feature flags for v0

- Everything always included
- Keep it simple
- Revisit if binary size becomes a concern

---

## Project Setup (Q53-Q55)

### Q53: Minimum Supported Rust Version (MSRV)

**Question:** What's the minimum Rust version we support?

**Options:**

- Latest stable only
- Stable minus 2 (e.g., if current is 1.75, support 1.73+)
- Specific version (e.g., 1.70+)

**Decision:** Latest stable only

- New project, no legacy users
- Simplifies CI
- Can pin later if users request

---

### Q54: CI/CD Setup

**Question:** What CI/CD do we use?

**Options:**

- GitHub Actions
- None for v0
- Other (GitLab CI, etc.)

**Decision:** GitHub Actions

- Run `cargo test` on PR
- Run `cargo clippy` for lints
- Auto-release binaries on git tag
- Test platforms: Linux x86_64, macOS arm64

---

### Q55: License

**Question:** What license?

**Options:**

- MIT
- Apache-2.0
- MIT OR Apache-2.0 (dual, Rust convention)
- GPL

**Decision:** MIT OR Apache-2.0 (dual license)

- Standard in Rust ecosystem
- Maximum compatibility
- Users choose which suits them

---

## Daemon Details (Q56-Q59)

### Q56: Daemonization Method

**Question:** How does daemon become a background process?

**Options:**

- fork() and detach (traditional Unix)
- Just run in background (user does `acd daemon &`)
- Tokio spawn (stay attached but async)
- systemd/launchd managed

**Decision:** `--daemonize` flag via `daemonize` crate

- `acd daemon` = foreground (for dev/debug, see logs)
- `acd daemon -d` = background (normal usage, forks and detaches)
- Handles fork, setsid, close fds automatically

---

### Q57: PID File

**Question:** Do we create a PID file?

**Options:**

- Yes: `~/.local/share/agent-console/daemon.pid`
- No: detect via socket existence
- Both: PID file + socket check

**Purpose:** Prevents multiple instances, allows `acd stop` to find daemon.

**Decision:** Both PID file + socket check

- PID file: `~/.local/share/agent-console/daemon.pid`
- Allows `acd stop` to send signal directly
- Socket check verifies daemon is actually responding
- Stale PID file (process dead) → clean up and allow start

---

### Q58: Multiple Daemon Instances

**Question:** What if user accidentally starts two daemons?

**Options:**

- Detect and refuse (check PID file or socket)
- Allow multiple (different sockets)
- Kill old, start new

**Decision:** Detect and refuse

- Try to connect to existing socket first
- If daemon running: "Daemon already running (PID 1234)"
- Prevents accidental multiple instances

---

### Q59: Daemon Health Check

**Question:** How to verify daemon is healthy?

**Options:**

- `acd status` command
- Ping/pong over socket
- Just check socket exists

**Decision:** `acd status` with ping/pong

- `acd status` command sends PING, expects PONG
- Shows: running/stopped, PID, uptime, connected dashboards count
- More reliable than just checking socket file exists

---

## Protocol Details (Q60-Q62)

### Q60: Message Framing

**Question:** How do we delimit messages over the socket?

**Options:**

- Newline-delimited JSON (one JSON object per line)
- Length-prefix (4-byte length + JSON body)
- Null-byte delimiter

**Decision:** Newline-delimited JSON

- One JSON object per line (JSON Lines format)
- Simple to implement and debug
- Our messages are simple, won't contain literal newlines

---

### Q61: Protocol Versioning

**Question:** How do we handle protocol changes?

**Options:**

- Version field in every message
- Handshake on connect (exchange versions)
- No versioning (breaking changes = major version bump)

**Decision:** Version field in every message

- Every message includes: `{"version": 1, "cmd": "...", ...}`
- Simpler than handshake (no special first-message logic)
- Daemon can check version on any message
- Helps catch mismatches during testing

---

### Q62: Backward Compatibility

**Question:** Old client with new daemon (or vice versa)?

**Options:**

- Strict: must match versions
- Lenient: ignore unknown fields, use defaults for missing
- Negotiated: handshake determines common features

**Decision:** Lenient

- Ignore unknown fields (forward compatible)
- Use defaults for missing fields (backward compatible)
- Only break on major structural changes

---

## Configuration Details (Q63-Q65)

### Q63: Unknown Config Keys

**Question:** What if config file has unknown keys?

**Options:**

- Ignore silently
- Warn but continue
- Error and refuse to start

**Decision:** Warn but continue

- Log warning: "Unknown config key: foo"
- Use rest of config normally
- Typos get noticed, old configs still work

---

### Q64: Environment Variable Overrides

**Question:** Can env vars override config file?

**Options:**

- Yes: `ACD_SOCKET_PATH`, `ACD_LOG_LEVEL`, etc.
- No: config file only
- Limited: only some settings

**Decision:** Yes, full override

- All settings overridable via env vars
- Useful for testing: `ACD_SOCKET_PATH=/tmp/test.sock acd daemon`
- Priority: env var > config file > default

---

### Q65: First Run Experience

**Question:** What happens on first launch (no config file)?

**Options:**

- Create default config file
- Work without config (use defaults)
- Interactive setup wizard

**Decision:** Work without config

- Zero friction to start
- Config file is optional, only for customization
- User creates config only when they want to change defaults

---

## Error Recovery (Q66-Q68)

### Q66: Disk Full

**Question:** What if we can't write to log/history file?

**Options:**

- Log to stderr, continue running
- Disable logging, continue
- Exit with error

**Decision:** Fallback to stderr, continue

- Daemon keeps running (main function still works)
- Log warning: "Cannot write to log file, falling back to stderr"
- User sees errors if running foreground

---

### Q67: Corrupt History File

**Question:** What if history.json is corrupted?

**Options:**

- Delete and start fresh
- Backup corrupt file, start fresh
- Exit with error, require manual fix

**Decision:** Backup and start fresh

- Rename to `history.json.corrupt.{timestamp}`
- Start with empty history
- Log warning so user knows what happened

---

### Q68: Socket Path Not Writable

**Question:** What if we can't create socket?

**Options:**

- Exit with clear error message
- Try fallback location
- Prompt user to fix permissions

**Decision:** Create parent dirs, then exit with clear error if fails

- Create parent directories if missing (`mkdir -p`)
- If still fails (permissions), exit with clear error
- No silent fallback (avoids confusion with multiple sockets)

---

## Integration Details (Q69-Q70)

### Q69: Hook Installation

**Question:** How do users install Claude Code hooks?

**Decision:** `acd hooks install` command with idempotent append

**Commands:**

- `acd hooks install` - adds hooks to Claude Code config
- `acd hooks uninstall` - removes only acd hooks
- `acd hooks status` - shows if hooks are configured

**Installation algorithm:**

```text
1. Read Claude Code settings.json
2. For each hook event we need (PreToolUse, Stop, SessionStart, SessionEnd):
   a. If event missing → create as array with our hook
   b. If event exists as object → convert to array, append our hook
   c. If event exists as array → append our hook
3. Before appending, check if our hook already exists (idempotent)
   - Match by command containing "acd set --claude-hook"
   - If found → skip (already installed)
   - If not found → append
4. Write updated config
```

**Example transformation:**

```json
// Before (user has single hook, not array - edge case)
{
  "hooks": {
    "PreToolUse": {
      "matcher": "Bash",
      "hooks": [{ "command": "user-script.sh" }]
    }
  }
}

// After (converted to array + our hook appended)
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [{ "command": "user-script.sh" }]
      },
      {
        "matcher": "*",
        "hooks": [{ "type": "command", "command": "acd set --claude-hook PreToolUse", "timeout": 5 }]
      }
    ]
  }
}
```

**Uninstall:** Remove only entries where command contains "acd set
--claude-hook". Preserve all other hooks.

---

### Q70: Multiple Claude Code Versions

**Question:** What if user has different Claude Code versions?

**Options:**

- Support latest only
- Detect version, adapt behavior
- Document minimum supported version

**Decision:** Support latest only

- Document: "Requires Claude Code 2.0.76+"
- Hook format unlikely to change drastically
- Keeps implementation simple

---

### Q71: Repository Structure

**Question:** How do we organize the codebase?

**Decision:** Cargo workspace with two crates

```text
agent-console-dashboard/
├── Cargo.toml                    # workspace root
├── crates/
│   ├── agent-console-dashboard/  # binary crate
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   └── claude-usage/             # library crate
│       ├── Cargo.toml
│       └── src/lib.rs
```

**Workspace Cargo.toml:**

```toml
[workspace]
members = ["crates/agent-console-dashboard", "crates/claude-usage"]
```

**Binary crate config:**

```toml
[package]
name = "agent-console-dashboard"

[[bin]]
name = "acd"
path = "src/main.rs"

[[bin]]
name = "agent-console-dashboard"
path = "src/main.rs"
```

**CLI commands:** Both `acd` and `agent-console-dashboard` work.

---

## Related Tools

Existing tools for Claude Code usage monitoring (discovered 2026-01-22):

### CLI Tools

| Tool                  | Language   | Description                                     | Link                                                          |
| --------------------- | ---------- | ----------------------------------------------- | ------------------------------------------------------------- |
| ccusage               | TypeScript | Analyzes local JSONL files, beautiful tables    | [GitHub](https://github.com/ryoppippi/ccusage)                |
| claude-code-usage-bar | Python     | Terminal statusline with token/cost display     | [GitHub](https://github.com/leeguooooo/claude-code-usage-bar) |
| claude-code-usage     | TypeScript | Lightweight local usage analysis                | [GitHub](https://github.com/evanlong-me/claude-code-usage)    |
| tokscale              | TypeScript | Multi-platform tracking with contribution graph | [GitHub](https://github.com/junhoyeo/tokscale)                |

### Dashboards (Web UI)

| Tool             | Description                                 | Link                                                     |
| ---------------- | ------------------------------------------- | -------------------------------------------------------- |
| cc-metrics       | Self-hosted dashboard, RethinkDB, WebSocket | [GitHub](https://github.com/lasswellt/cc-metrics)        |
| claude-code-otel | OpenTelemetry observability dashboard       | [GitHub](https://github.com/ColeMurray/claude-code-otel) |
| sniffly          | Web dashboard by Chip Huyen                 | [GitHub](https://github.com/chiphuyen/sniffly)           |

### Native Apps

| Tool                 | Platform | Description                  | Link                                                             |
| -------------------- | -------- | ---------------------------- | ---------------------------------------------------------------- |
| Claude-Usage-Tracker | macOS    | Menu bar app (Swift/SwiftUI) | [GitHub](https://github.com/hamed-elfayome/Claude-Usage-Tracker) |

### Gap Identified → Spin-off Project

**No simple cross-platform library exists** that:

1. Handles credential fetching (macOS Keychain, Linux secret-service, etc.)
2. Calls the OAuth usage API
3. Returns structured JSON

**Spin-off project: `claude-usage`**

| Aspect    | Decision                                            |
| --------- | --------------------------------------------------- |
| Language  | Rust (reusable in acd, publishable as crate)        |
| Scope     | Credential handling + API call only                 |
| API       | Single function: `get_usage() -> Result<UsageData>` |
| Platforms | macOS (Keychain), Linux (secret-service or file)    |
| Security  | Read token → call API → discard token immediately   |
| Output    | Structured data (not just JSON string)              |

**Publishing roadmap:**

| Version | Registry  | Method                         |
| ------- | --------- | ------------------------------ |
| v0-v1   | crates.io | Rust crate                     |
| v1-v2   | npm       | napi-rs (native Node.js addon) |

**Benefits:**

- Reusable by acd and other tools
- Fills ecosystem gap
- Isolates credential handling (security boundary)
- Published to crates.io (npm later via napi-rs)

---

## External Requests

Feature requests submitted to other projects:

| Date       | Project                                                                        | Issue                                                                                               | Status |
| ---------- | ------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------- | ------ |
| 2026-01-22 | [Claude-Usage-Tracker](https://github.com/hamed-elfayome/Claude-Usage-Tracker) | [#90 - Historical usage tracking](https://github.com/hamed-elfayome/Claude-Usage-Tracker/issues/90) | Open   |

---

## Open - Needs Research or Decision

### Q72: Linux Credential Storage

**Question:** Where does Claude Code store OAuth credentials on Linux?

**Decision:** File-based storage at `~/.claude/.credentials.json`

**Platform comparison:**

| Platform | Storage   | Location                      |
| -------- | --------- | ----------------------------- |
| macOS    | Keychain  | `"Claude Code-credentials"`   |
| Linux    | JSON file | `~/.claude/.credentials.json` |

**File format (same as macOS Keychain value):**

```json
{
  "claudeAiOauth": {
    "accessToken": "sk-ant-oat01-...",
    "refreshToken": "sk-ant-ort01-...",
    "expiresAt": 1748658860401,
    "scopes": ["user:inference", "user:profile"]
  }
}
```

**Environment variable override:** `CLAUDE_CODE_OAUTH_TOKEN` takes precedence if
set.

**Implementation:**

```rust
// Pseudocode for cross-platform credential retrieval
fn get_oauth_token() -> Result<String> {
    // 1. Check env var first
    if let Ok(token) = env::var("CLAUDE_CODE_OAUTH_TOKEN") {
        return Ok(token);
    }

    // 2. Platform-specific
    #[cfg(target_os = "macos")]
    return read_from_keychain("Claude Code-credentials");

    #[cfg(target_os = "linux")]
    return read_json_file("~/.claude/.credentials.json");
}
```

**Note:** Linux storage is less secure than macOS Keychain (plain file on disk).
Credentials module should still follow Q34 security requirements (read → use →
discard immediately).

---

### Q73: Hook Stdin Data

**Question:** Should `acd set` parse stdin JSON from Claude Code, or just use
command-line flags?

**Decision:** Parse stdin JSON + use `--source` flag for agent type

**Rationale:** Stdin JSON contains `hook_event_name`, so we don't need a
separate event flag. But we need `--source` to know which parser to use (for
future multi-agent support).

**Data flow:**

```text
Claude Code hook fires
    ↓
stdin: { "session_id": "abc", "cwd": "/path", "hook_event_name": "PreToolUse", "tool_name": "..." }
    ↓
acd set --source claude-code
    ↓
Parse stdin JSON (using claude-code parser), extract:
  - session_id
  - cwd
  - hook_event_name → map to status
  - tool_name → check for AskUserQuestion
    ↓
Send to daemon: { session_id, cwd, status }
```

**Process lifetime:** `acd set` is short-lived (~1ms). Parses stdin, sends to
daemon, exits. No RAM accumulation.

**Multi-agent support (future):**

| Agent       | Command                        |
| ----------- | ------------------------------ |
| Claude Code | `acd set --source claude-code` |
| Gemini CLI  | `acd set --source gemini`      |
| Codex CLI   | `acd set --source codex`       |

Each source has its own parser extracting same output fields from different
input formats.

**Hook configuration (simplified):**

```json
{
  "hooks": {
    "PreToolUse": [{ "command": "acd set --source claude-code" }],
    "Stop": [{ "command": "acd set --source claude-code" }],
    "SessionStart": [{ "command": "acd set --source claude-code" }],
    "SessionEnd": [{ "command": "acd set --source claude-code" }]
  }
}
```

**If stdin is empty or invalid:**

- Log warning
- Exit 1 (non-blocking error per Q50)
- Daemon won't receive update, but Claude continues

---

### Q74: Question Status Detection

**Question:** How do we detect when Claude asks user a question (AskUserQuestion
tool)?

**Decision:** Check `tool_name` field in PreToolUse stdin JSON

**Stdin JSON for PreToolUse:**

```json
{
  "session_id": "abc123",
  "cwd": "/path/to/project",
  "hook_event_name": "PreToolUse",
  "tool_name": "AskUserQuestion",
  "tool_input": {
    "question": "Which approach should I use?"
  }
}
```

**Detection logic:**

```rust
// In acd set --claude-hook PreToolUse
if stdin.tool_name == "AskUserQuestion" {
    status = Status::Question;
} else {
    status = Status::Working;
}
```

**Updated hook mapping (revises Q50):**

| Hook Event   | tool_name       | Status    |
| ------------ | --------------- | --------- |
| PreToolUse   | AskUserQuestion | question  |
| PreToolUse   | (any other)     | working   |
| Stop         | -               | attention |
| SessionEnd   | -               | closed    |
| SessionStart | -               | working   |

**Alternative considered:** Dedicated hook with matcher (rejected - simpler to
check tool_name in code)

---

### Q75: TUI Color Scheme

**Question:** What colors for each status? Support light/dark terminal themes?

**Decision:** Semantic colors, theme support in v1+

| Status    | Color   | Meaning              |
| --------- | ------- | -------------------- |
| Working   | Blue    | Active, processing   |
| Attention | Yellow  | Needs user attention |
| Question  | Magenta | Awaiting user input  |
| Closed    | Gray    | Inactive             |
| Idle      | Gray    | No activity > 100m   |

**Theme support:**

- v0: Single color scheme (works on both dark/light terminals)
- v1+: Auto-detect or config-based theme selection
- v1+: Grayscale fallback for accessibility

---

### Q76: TUI Layout Responsiveness

**Question:** How to fit N sessions horizontally when space is limited?

**Decision:** Pagination with hidden count indicators

**Layout formula (horizontal, one-line widget):**

```text
Per active session: name + space + mm:ss + separator = name + 8 chars
Per idle session:   name + separator = name + 3 chars
Pagination: "← N+ | " and " | M+ →" when items hidden
```

**Algorithm:**

1. Sort sessions: active first (by idle time, larger to smaller), then idle
   (>100m threshold)
2. Fit as many sessions as possible in available width
3. Show hidden count on each side: `← 3+ |` and `| 5+ →`
4. Hide arrow when no items in that direction

**Example:**

```text
← 3+ | my-project 05:23 | [api-server] 03:15 | dashboard | 5+ →
      ^^^^^^^^^^^^^^^^   ^^^^^^^^^^^^^^^^^^   ^^^^^^^^^
      active (blue)      selected (inverse)   idle (gray)
```

**Navigation:** h/l or ←/→ shifts by 1 item at a time (not whole page)

**Name truncation:** Dynamic based on session count and available width (see
Q42)

**v1+ feature:** Custom session ordering (user-defined priority)

---

### Q77: TUI Status Indicators

**Question:** Icons/symbols for each status? Unicode symbols or ASCII fallback?

**Decision:** Color only (no icons) - saves 2 chars per session

**Color scheme (from Q75):**

| Status    | Color   | Time display |
| --------- | ------- | ------------ |
| Working   | Blue    | mm:ss        |
| Attention | Yellow  | mm:ss        |
| Question  | Magenta | mm:ss        |
| Closed    | Gray    | hidden       |
| Idle      | Gray    | hidden       |

**Idle detection:**

- Threshold: 100 minutes (configurable)
- When idle time > threshold: change to gray, hide time display
- Saves space for active sessions

**Time format:** `mm:ss` (5 chars max: `99:59`)

**Single-color/grayscale fallback:** Defer to v1+ (configurable theme)

---

### Q78: TUI Session List Overflow

**Question:** When sessions exceed visible space, scroll or paginate?

**Decision:** Pagination with hidden count indicators (see Q76)

**Layout:**

```text
← 3+ | my-project 05:23 | api-server 03:15 | 5+ →
```

**Indicator visibility:**

- `← N+`: shown when N items hidden to left, hidden when at start
- `M+ →`: shown when M items hidden to right, hidden when at end

**Navigation:**

- h/l or ←/→ moves selection by 1 item
- Viewport auto-scrolls to keep selection visible
- Same behavior for both Line 1 (sessions) and Line 2 (history)

**Note:** Same pagination pattern used for history line (Q93)

---

### Q79: TUI Keyboard Shortcuts

**Question:** What are all keyboard shortcuts? Vim-style, Emacs-style, or
custom?

**Decision:** Vim-style + arrow keys + mouse support

**Key bindings:**

| Key   | Action                           |
| ----- | -------------------------------- |
| h / ← | Navigate left                    |
| l / → | Navigate right                   |
| j / ↓ | Navigate down (Line 1 → Line 2)  |
| k / ↑ | Navigate up (Line 2 → Line 1)    |
| Enter | Action on selected (Line 1 only) |
| Esc   | Deselect / clear focus           |
| q     | Quit dashboard                   |
| ?     | Toggle help overlay              |

**Mouse:**

- Click to select item (same as keyboard navigation)
- Hover over session to focus it

**Focus behavior:**

- Terminal focused: inverse background on selected item
- Terminal unfocused: no highlight (all items normal style)
- Focus state persists across terminal focus changes

See Q95 for selectability and actions per line.

---

### Q80: TUI Focus/Selection Highlight

**Question:** How to visually indicate selected/focused session?

**Decision:** Inverse background color

**Example:**

```text
my-project 05:23 | [api-server 03:15] | dashboard
                   ^^^^^^^^^^^^^^^^^^
                   inverse background (focused)
```

**Implementation:** Same status color but with background filled, text color
inverted (black or white depending on background brightness)

---

### Q81: TUI Animation/Transitions

**Question:** Any animations (spinner for working, fade for closed)?

**Decision:** None (static display)

**Rationale:**

- Simpler implementation
- Less CPU usage
- Avoids terminal flicker issues
- Color already indicates status clearly

---

### Q82: TUI Error Display

**Question:** Where and how to show errors (daemon disconnect, API failure)?

**Decision:** Status bar (replace content with error message)

**Example:**

```text
Normal:  my-project 05:23 | api-server 03:15 | dashboard
Error:   ⚠ Daemon disconnected - reconnecting...
```

**Behavior:**

- Error replaces session list temporarily
- Returns to normal display when error resolves
- Use warning color (yellow) for error text

---

### Q83: TUI Help/Legend

**Question:** Show keyboard shortcut help? Always visible or toggle?

**Decision:** Toggle with `?` key

**Behavior:**

- Press `?` to show help overlay
- Press any key to dismiss
- Overlay shows all keybindings

**Help overlay content:**

```text
┌─────────────────────────────┐
│ Keyboard Shortcuts          │
│                             │
│ h/←  Previous session       │
│ l/→  Next session           │
│ Enter  Expand details       │
│ q  Quit                     │
│ ?  Toggle this help         │
│                             │
│ Press any key to close      │
└─────────────────────────────┘
```

---

## API Usage Widget (Q84-Q88)

### Q84: Usage Widget - Periods

**Question:** Which usage periods to display?

**Decision:** Both 5h and 7d (space available on bottom line)

---

### Q85: Usage Widget - Format

**Question:** How to display usage data?

**Decision:** Percentage + time elapsed

**Format:** `usage% / time%`

**Example:**

```text
5h: 42% / 50%  7d: 77% / 43%
    ^^^   ^^^
    used  time elapsed
```

**Interpretation:** "Used 42% of quota, 50% of time period passed" → pacing well

---

### Q86: Usage Widget - Refresh Rate

**Question:** How often to fetch usage data?

**Decision:** Configurable, default 180s

| Setting | Value                                 |
| ------- | ------------------------------------- |
| Default | 180 seconds (3 min = 1% of 5h period) |
| Minimum | 1 second                              |
| Maximum | 3600 seconds (1 hour)                 |
| Type    | Positive integer                      |
| Invalid | Warning + use default                 |

---

### Q87: Usage Widget - Position

**Question:** Where in layout?

**Decision:** Bottom left

**Layout:**

```text
Line 1: ← my-project 05:23 | api-server 03:15 | dashboard →
Line 2: 5h: 42% / 50%  7d: 77% / 43%
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
        bottom left
```

---

### Q88: Usage Widget - Label Toggle

**Question:** Can labels be hidden?

**Decision:** v1+ feature

- Labels ("5h:", "7d:") visible by default
- v1+: Config option to hide labels when space tight
- Compact mode: `42%/50%  77%/43%`

---

## Edge Cases (Q89-Q92)

### Q89: Usage Widget - Credentials Missing

**Question:** What to show when Claude Code credentials missing or expired?

**Decision:** Show login message

**Display:** `Login to Claude Code`

---

### Q90: Usage Widget - Over 100%

**Question:** What to display when utilization exceeds 100% (extra usage
enabled)?

**Decision:** Show actual percentage + warning color

**Display:** `105%` in red/warning color

**Rationale:** API returns actual value (can be 102%, 105%, etc.). No cap
needed. Color draws attention to overage.

---

### Q91: Pagination Order Stability

**Question:** When sessions sorted by activity, does order update during
pagination?

**Decision:** Stable order for v0, dynamic reorder in v1+

**v0 behavior:** Order stays fixed during session. No items shifting
unexpectedly while paginating.

**v1+ feature:** Option to enable dynamic reorder as sessions become idle.

---

### Q92: Time Elapsed - Clock Skew

**Question:** What if system clock is wrong, causing negative or >100% time
elapsed?

**Decision:** Show actual calculated value (expose clock issue)

**Rationale:** Weird percentages (-5% or 150%) signal to user their clock is
wrong. More transparent than silent clamping.

---

### Q93: Session Widget - History Line

**Question:** What does Line 2 of the session widget show?

**Decision:** Status history of selected session with same pagination pattern

**Format:** `status duration` entries, oldest to newest

**Example (2-line session widget):**

```text
Line 1: ← 3+ | my-project 05:23 | [api-server] 03:15 | 5+ →
Line 2: ← 2+ | working 10m | attention 2m | question 30s | 1+ →
              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
              history of selected session (api-server)
```

**Pagination:** Same as session line - arrows with hidden count, shift by 1 item

**When no session selected:** Empty or show hint

**v1+ consideration:** Global status feed showing changes from all sessions

---

### Q94: Dashboard Layout Options

**Question:** How are widgets arranged?

**Decision:** Two layout options

**2-line layout:**

```text
Line 1: ← 3+ | my-project 05:23 | [api-server] 03:15 | 5+ →   (session widget, 1-line)
Line 2: 5h: 42% / 50%  7d: 77% / 43%                          (usage widget)
```

**3-line layout (default):**

```text
Line 1: ← 3+ | my-project 05:23 | [api-server] 03:15 | 5+ →   (session widget line 1)
Line 2: ← 2+ | working 10m | attention 2m | question 30s →    (session widget line 2: history)
Line 3: 5h: 42% / 50%  7d: 77% / 43%                          (usage widget)
```

**Session widget versions:**

| Version | Lines | Content                             |
| ------- | ----- | ----------------------------------- |
| 1-line  | 1     | Session names with pagination       |
| 2-line  | 2     | Names + history of selected session |

**Default:** 3-line layout (2-line session widget + usage widget)

---

### Q95: Line Selectability and Actions

**Question:** What's selectable on each line? What actions are available?

**Decision:** Both lines selectable, actions only on Line 1

| Line   | Content  | Selectable | Enter action     | Mouse click |
| ------ | -------- | ---------- | ---------------- | ----------- |
| Line 1 | Sessions | Yes        | Expand/resurrect | Select      |
| Line 2 | History  | Yes        | None             | Select      |

**Rationale:**

- Consistent navigation (focus can move anywhere)
- History is informational, no meaningful action
- Selection on Line 2 is visual feedback only

---

### Q96: History in 2-Line Layout

**Question:** Where does history show in 2-line layout (no dedicated history
line)?

**Decision:** History replaces usage widget when session selected

**2-line layout behavior:**

```text
No selection:
  Line 1: ← 3+ | my-project 05:23 | api-server 03:15 | 5+ →
  Line 2: 5h: 42% / 50%  7d: 77% / 43%                        (usage widget)

Session selected:
  Line 1: ← 3+ | my-project 05:23 | [api-server] 03:15 | 5+ →
  Line 2: ← 2+ | working 10m | attention 2m | question 30s →  (history replaces usage)
```

**Mouse hover:** Hovering over a session focuses it (same as keyboard
navigation)

**v1+ feature:** 2-line session widget option (dedicated history line without
replacing usage)

---

### Q97: Deselect in 2-Line Layout

**Question:** How to return to usage widget after selecting a session?

**Decision:** Escape to deselect + auto-deselect on terminal unfocus

**Behavior:**

- Escape clears visual highlight → history replaced by usage widget
- Terminal unfocus also clears visual highlight → usage widget visible
- Position remembered in memory (see Q102)
- Works in both 2-line and 3-line layouts

**Rationale:** When user switches away, they probably want to glance at usage,
not history of a specific session.

---

### Q98: Line 2 Content by Selection (3-Line Layout)

**Question:** What does Line 2 show based on Line 1 selection?

**Decision:** Depends on what's selected on Line 1

**Line 2 behavior:**

| Line 1 selection | Line 2 shows                        |
| ---------------- | ----------------------------------- |
| Global item [🌐] | Global activity feed (all sessions) |
| Session          | History of selected session         |

**Global feed format:**

```text
← 2+ | my-project → attention | api-server → working | dashboard → closed →
       ^^^^^^^^^^^^^^^^^^^^^^   ^^^^^^^^^^^^^^^^^^^^^   ^^^^^^^^^^^^^^^^^^^
       session: new status      session: new status     session: new status
```

**See Q100 for global item details.**

---

### Q99: Escape While on Line 2

**Question:** Where does focus go when Esc pressed while on Line 2?

**Decision:** Focus moves to Line 1 (first visible session)

**Behavior:**

- Esc on Line 2 → focus jumps to Line 1, first visible session selected
- Line 2 becomes global feed (3-line) or usage widget (2-line)
- Global feed is display-only, not navigable

**Note:** To fully deselect (no focus), press Esc again on Line 1.

---

### Q100: Global Item on Line 1

**Question:** How to access global feed via keyboard?

**Decision:** Add "global" item as first position on Line 1

**Layout:**

```text
Line 1: ← [🌐] | my-project 05:23 | api-server 03:15 | 5+ →
           ^^^
           global item (first position)
```

**Fallback:** `[G]` when emoji not supported

**Behavior:**

- Global item always at first position
- Select global → Line 2 shows global feed
- Can navigate to Line 2 (j/k) when global selected
- Enter on global feed item → selects that session on Line 1

**Updates Q98:** Global feed shown when global item selected (not "no
selection")

**Navigation flow:**

```text
1. Start with global item selected (default)
2. l/→ to navigate to sessions
3. h/← to return to global item
4. j/↓ when global selected → navigate global feed on Line 2
5. Enter on global feed item → jump to that session
```

---

### Q101: Help Overlay Close Keys

**Question:** How to close help overlay?

**Decision:** Any key closes help (including Esc)

**Behavior:**

- Help open → any keypress closes help (key not processed further)
- After help closes, next Esc deselects as normal
- `?` toggles: open when closed, close when open

**Rationale:** Simple mental model - help is modal, dismiss with any key to
return to normal operation.

---

### Q102: Focus State on Terminal Refocus

**Question:** What's selected when terminal regains focus?

**Decision:** Previous selection restored

**Focus state management:**

- Store last focused position in dashboard memory (not persisted to file)
- Terminal unfocus: visual highlight removed, position remembered
- Terminal refocus: restore previous selection
- Esc deselect: highlight removed, position still remembered
- Next navigation key: regain focus at remembered position

**Initial state (app start):** Global item selected

**Rationale:** User expects to continue where they left off. No need to persist
focus across app restarts.

---

### Q103: Global Item in 2-Line Layout

**Question:** What does Line 2 show when global item selected in 2-line layout?

**Decision:** Global feed (same as 3-line layout)

**2-line layout Line 2 behavior:**

| Selection state           | Line 2 shows                |
| ------------------------- | --------------------------- |
| Global item [🌐]          | Global activity feed        |
| Session                   | History of selected session |
| Deselected (no highlight) | Usage widget                |

**Consistency:** Global item behavior same across layouts. Usage widget only
visible when deselected in 2-line layout.

---

### Q104: Enter Action - Switch to Session Tab

**Question:** What happens when pressing Enter on an already-focused session?

**Decision:** Switch to that session's terminal tab (external callback)

**Behavior:**

- First selection (click/navigate): Focus session, Line 2 shows history
- Enter on focused session: Execute callback to switch terminal tab

**Zellij integration (built-in):**

```bash
# Switch to tab by name
zellij action go-to-tab-name "session-display-name"
```

**Tab name mapping:**

| Config         | Behavior                                    |
| -------------- | ------------------------------------------- |
| None (default) | Tab name = session display name             |
| Custom mapping | Config file maps session names to tab names |

**Config example:**

```toml
[tab_mapping]
# session_display_name = "tab_name"
my-project = "my-project"  # same name (default behavior)
api-server = "backend"     # custom mapping
```

**Custom callback (for other multiplexers):**

```toml
[integration]
switch_tab_command = "tmux select-window -t {tab_name}"
```

**Placeholder:** `{tab_name}` replaced with mapped tab name

---

### Q105: Sound/Notification on Status Change

**Question:** Should we play sound or show notification when status changes?

**Decision:** Deferred to v1+

---

### Q106: CLI Help and TUI Help Panel

**Question:** What help is shown where?

**Decision:** Separate help for CLI and TUI

**CLI help (`acd --help` or `acd -h`):**

- Shows all subcommands and options
- Standard clap-generated help

**TUI help panel (`?` key):**

- Shows hotkeys only (navigation, actions)
- Does NOT show CLI subcommands

**Hotkeys shown in help panel:**

| Key   | Action                |
| ----- | --------------------- |
| h / ← | Navigate left         |
| l / → | Navigate right        |
| j / ↓ | Navigate down         |
| k / ↑ | Navigate up           |
| Enter | Action on selected    |
| Esc   | Deselect / close help |
| q     | Quit                  |
| ?     | Toggle this help      |

---

## Parking Lot

Questions deferred for later:

- Multi-user support (probably never)
- Remote access (probably never)
- Plugin system for custom widgets
- Integration with other AI agents (after v1)

---

## Resolution Tracking

| Question | Status    | Decision                                       | Date       |
| -------- | --------- | ---------------------------------------------- | ---------- |
| Q1       | Resolved  | Single Daemon                                  | 2026-01-17 |
| Q2       | Resolved  | Auto-start if socket doesn't exist             | 2026-01-17 |
| Q3       | Resolved  | XDG: ~/.config/agent-console/                  | 2026-01-17 |
| Q4       | Resolved  | Keep 4 statuses as C-like enum                 | 2026-01-17 |
| Q5       | Revised   | Dedicated history space, no TTL                | 2026-01-18 |
| Q6       | Resolved  | Anthropic Usage API (v0 core)                  | 2026-01-17 |
| Q7       | Resolved  | PreToolUse + AskUserQuestion (v2.0.76+)        | 2026-01-17 |
| Q8       | Deferred  | Zellij plugin (v2)                             | 2026-01-17 |
| Q9       | Deferred  | Tmux plugin (on request only)                  | 2026-01-17 |
| Q10      | Resolved  | 3 widgets, config file only                    | 2026-01-17 |
| Q11      | Resolved  | Immediate mode (Ratatui standard)              | 2026-01-17 |
| Q12      | Resolved  | `acd` / `agent-console-dashboard`              | 2026-01-17 |
| Q13      | N/A       | No migration (cc-hub never released)           | 2026-01-17 |
| Q14      | Resolved  | Platform-specific socket location              | 2026-01-17 |
| Q15      | Resolved  | JSON (MessagePack as alternative)              | 2026-01-17 |
| Q16      | Resolved  | CC session_id + full payload each msg          | 2026-01-17 |
| Q17      | Resolved  | Auto-create on first message                   | 2026-01-17 |
| Q18      | Resolved  | SessionEnd hook → Closed status                | 2026-01-17 |
| Q19      | Resolved  | Working directory only (session_id unused)     | 2026-01-17 |
| Q20      | Resolved  | Hybrid: events + low-accuracy tick             | 2026-01-17 |
| Q21      | Resolved  | Both mouse and keyboard                        | 2026-01-17 |
| Q22      | Resolved  | Multiple dashboards allowed                    | 2026-01-17 |
| Q23      | Deferred  | Windows (Named Pipes) in v2+                   | 2026-01-17 |
| Q24      | Resolved  | Basic recovery (options 1+2), auto-restart v2+ | 2026-01-17 |
| Q25      | Resolved  | Multiple: stop cmd, SIGTERM, auto-stop         | 2026-01-17 |
| Q26      | Resolved  | SIGTERM/SIGINT=shutdown, SIGHUP=reload         | 2026-01-17 |
| Q27      | Resolved  | Hot reload in v0 (SIGHUP or `acd reload`)      | 2026-01-17 |
| Q28      | Resolved  | "No active sessions. See README for setup."    | 2026-01-17 |
| Q29      | Resolved  | 5s timeout, exit 1 on failure                  | 2026-01-22 |
| Q30      | Resolved  | Return error, keep connection open             | 2026-01-18 |
| Q31      | Resolved  | Auto-start + 3 retries with backoff            | 2026-01-18 |
| Q32      | Resolved  | Stale data + background retry                  | 2026-01-18 |
| Q33      | Resolved  | User-only 0600                                 | 2026-01-18 |
| Q34      | Resolved  | Reuse Claude Code OAuth credentials            | 2026-01-22 |
| Q35      | Resolved  | Basic length limits only                       | 2026-01-18 |
| Q36      | Resolved  | Soft limit warning at 50                       | 2026-01-18 |
| Q37      | Resolved  | Soft limit warning at 50                       | 2026-01-18 |
| Q38      | Resolved  | Last 200 AND within 24h                        | 2026-01-18 |
| Q39      | Resolved  | Defaults documented                            | 2026-01-18 |
| Q40      | Duplicate | See Q28                                        | 2026-01-18 |
| Q41      | Resolved  | Parent dir or session ID suffix                | 2026-01-18 |
| Q42      | Resolved  | Systematic cascade, max 32 chars               | 2026-01-18 |
| Q43      | Resolved  | Full unicode support                           | 2026-01-18 |
| Q44      | Resolved  | Cargo + binaries v0, Homebrew v1+              | 2026-01-18 |
| Q45      | Resolved  | Built-in via clap_complete                     | 2026-01-18 |
| Q46      | Deferred  | Man pages v1+                                  | 2026-01-18 |
| Q47      | Resolved  | File logging in ~/.local/share/                | 2026-01-18 |
| Q48      | Resolved  | Unit + integration + e2e tests                 | 2026-01-18 |
| Q49      | Resolved  | No feature flags for v0                        | 2026-01-18 |
| Q50      | Resolved  | Hook contract documented                       | 2026-01-22 |
| Q51      | Resolved  | Track duration for all statuses                | 2026-01-18 |
| Q52      | Resolved  | Always show session directory                  | 2026-01-18 |
| Q53      | Resolved  | Latest stable Rust only                        | 2026-01-18 |
| Q54      | Resolved  | GitHub Actions                                 | 2026-01-18 |
| Q55      | Resolved  | MIT OR Apache-2.0 dual                         | 2026-01-18 |
| Q56      | Resolved  | --daemonize flag via crate                     | 2026-01-18 |
| Q57      | Resolved  | Both PID file + socket check                   | 2026-01-18 |
| Q58      | Resolved  | Detect and refuse                              | 2026-01-18 |
| Q59      | Resolved  | acd status with ping/pong                      | 2026-01-18 |
| Q60      | Resolved  | Newline-delimited JSON                         | 2026-01-18 |
| Q61      | Resolved  | Version field in every message                 | 2026-01-18 |
| Q62      | Resolved  | Lenient (ignore unknown, defaults)             | 2026-01-18 |
| Q63      | Resolved  | Warn but continue                              | 2026-01-18 |
| Q64      | Resolved  | Yes, full env var override                     | 2026-01-18 |
| Q65      | Resolved  | Work without config                            | 2026-01-18 |
| Q66      | Resolved  | Fallback to stderr, continue                   | 2026-01-18 |
| Q67      | Resolved  | Backup corrupt file, start fresh               | 2026-01-18 |
| Q68      | Resolved  | mkdir -p, then clear error                     | 2026-01-18 |
| Q69      | Resolved  | acd hooks install (idempotent)                 | 2026-01-22 |
| Q70      | Resolved  | Support latest CC only                         | 2026-01-18 |
| Q71      | Resolved  | Workspace structure (two crates)               | 2026-01-22 |
| Q72      | Resolved  | File: ~/.claude/.credentials.json              | 2026-01-22 |
| Q73      | Resolved  | Parse stdin + --source flag                    | 2026-01-22 |
| Q74      | Resolved  | Check tool_name in PreToolUse stdin            | 2026-01-22 |
| Q75      | Resolved  | Semantic colors, theme v1+                     | 2026-01-22 |
| Q76      | Resolved  | Overflow indicator + sorted by activity        | 2026-01-22 |
| Q77      | Resolved  | Color only, no icons                           | 2026-01-22 |
| Q78      | Resolved  | Pagination with arrows                         | 2026-01-22 |
| Q79      | Resolved  | Vim + arrow keys                               | 2026-01-22 |
| Q80      | Resolved  | Inverse background                             | 2026-01-22 |
| Q81      | Resolved  | None (static)                                  | 2026-01-22 |
| Q82      | Resolved  | Status bar replace content                     | 2026-01-22 |
| Q83      | Resolved  | Toggle with ? key                              | 2026-01-22 |
| Q84      | Resolved  | Both 5h and 7d                                 | 2026-01-22 |
| Q85      | Resolved  | Percentage + time elapsed                      | 2026-01-22 |
| Q86      | Resolved  | Default 180s, limits 1-3600                    | 2026-01-22 |
| Q87      | Resolved  | Bottom left                                    | 2026-01-22 |
| Q88      | Resolved  | Label toggle v1+                               | 2026-01-22 |
| Q89      | Resolved  | Show "Login to Claude Code"                    | 2026-01-22 |
| Q90      | Resolved  | Show actual % + warning color                  | 2026-01-22 |
| Q91      | Resolved  | Stable order v0, dynamic v1+                   | 2026-01-22 |
| Q92      | Resolved  | Show actual value (expose clock issue)         | 2026-01-22 |
| Q93      | Resolved  | History of selected session                    | 2026-01-22 |
| Q94      | Resolved  | 2-line or 3-line layout (default: 3-line)      | 2026-01-22 |
| Q95      | Resolved  | Both lines selectable, actions on Line 1 only  | 2026-01-22 |
| Q96      | Resolved  | History replaces usage when selected (2-line)  | 2026-01-22 |
| Q97      | Resolved  | Escape to deselect                             | 2026-01-22 |
| Q98      | Resolved  | Global activity feed when no selection         | 2026-01-22 |
| Q99      | Resolved  | Esc on Line 2 → focus to Line 1                | 2026-01-22 |
| Q100     | Resolved  | Global item [🌐] at first position             | 2026-01-22 |
| Q101     | Resolved  | Both ? and Esc close help overlay              | 2026-01-22 |
| Q102     | Resolved  | Previous selection restored on refocus         | 2026-01-22 |
| Q103     | Resolved  | Global item shows feed in 2-line layout too    | 2026-01-22 |
| Q104     | Resolved  | Enter on focused session → switch tab          | 2026-01-22 |
| Q105     | Deferred  | Sound/notification v1+                         | 2026-01-22 |
| Q106     | Resolved  | CLI help via --help, TUI help shows hotkeys    | 2026-01-22 |
