# E2E Test Plan

End-to-end testing procedure for Agent Console Dashboard covering installation,
hook integration, daemon lifecycle, and UI validation.

## Prerequisites

Before running this test plan, verify the following are installed:

- Rust toolchain (1.70+)
- cargo
- Claude Code installed and accessible

Verify with:

```sh
rustc --version
cargo --version
which claude
```

## Test Procedure

### Step 1: Build and Install

Build and install the `acd` binary to make it available in PATH.

```sh
cargo install --path crates/agent-console-dashboard
```

Expected output:

```text
  Installing agent-console-dashboard v0.1.0 (...)
   Installed package `agent-console-dashboard v0.1.0` (executable `acd`)
```

Verify installation:

```sh
which acd
```

Should output a path like `/Users/<username>/.cargo/bin/acd`.

### Step 2: Plugin Setup

Install ACD hooks into Claude Code settings.

```sh
acd install
```

Expected output:

```text
  Installed: SessionStart -> acd claude-hook attention
  Installed: UserPromptSubmit -> acd claude-hook working
  Installed: Stop -> acd claude-hook attention
  Installed: SessionEnd -> acd claude-hook closed
  Installed: Notification (elicitation_dialog) -> acd claude-hook question
  Installed: Notification (permission_prompt) -> acd claude-hook attention

Hooks: 6 installed, 0 already present, 0 errors

Restart Claude Code for hooks to take effect.
```

Verify hook installation:

```sh
cat ~/.claude/settings.json
```

Should show a `"hooks"` section with 6 ACD hook entries.

### Step 3: Start Daemon

Start the daemon in foreground mode with debug logging.

Terminal 1:

```sh
ACD_LOG=debug acd daemon
```

Expected output:

```text
[INFO] Agent Console Daemon starting...
[INFO] Listening on /tmp/agent-console-dashboard.sock
```

The daemon will remain running in this terminal.

Alternative: lazy-start via hooks (daemon will start automatically when first
hook fires). Skip to Step 5 if using lazy-start approach.

### Step 4: Verify Daemon Status

In a new terminal, check daemon health.

Terminal 2:

```sh
acd status
```

Expected output:

```text
Agent Console Daemon
  Status:      running
  Uptime:      0m 15s
  Sessions:    0 active, 0 closed
  Connections: 0 dashboards
  Memory:      2.4 MB
  Socket:      /tmp/agent-console-dashboard.sock
```

### Step 5: Simulate Hook Events

Simulate Claude Code hook calls by sending JSON via stdin.

Create a test session:

```sh
echo '{"session_id":"test-session-001","cwd":"/tmp/test-project","transcript_path":"/tmp/transcript","permission_mode":"default","hook_event_name":"SessionStart"}' | acd claude-hook working
```

Expected output:

```json
{ "continue": true }
```

Send status transitions:

```sh
# Transition to attention
echo '{"session_id":"test-session-001","cwd":"/tmp/test-project","transcript_path":"/tmp/transcript","permission_mode":"default","hook_event_name":"Stop"}' | acd claude-hook attention

# Transition to question
echo '{"session_id":"test-session-001","cwd":"/tmp/test-project","transcript_path":"/tmp/transcript","permission_mode":"default","hook_event_name":"Notification"}' | acd claude-hook question

# Transition back to working
echo '{"session_id":"test-session-001","cwd":"/tmp/test-project","transcript_path":"/tmp/transcript","permission_mode":"default","hook_event_name":"UserPromptSubmit"}' | acd claude-hook working

# Close session
echo '{"session_id":"test-session-001","cwd":"/tmp/test-project","transcript_path":"/tmp/transcript","permission_mode":"default","hook_event_name":"SessionEnd"}' | acd claude-hook closed
```

All commands should return `{"continue": true}`.

### Step 6: Verify Session State

Dump daemon state to verify session was created and tracked.

```sh
acd dump
```

Expected output (JSON):

```json
{
  "sessions": [
    {
      "session_id": "test-session-001",
      "status": "closed",
      "working_dir": "/tmp/test-project",
      "created_at": "2026-02-11T12:34:56Z",
      "updated_at": "2026-02-11T12:35:10Z",
      "history": [
        { "status": "working", "timestamp": "2026-02-11T12:34:56Z" },
        { "status": "attention", "timestamp": "2026-02-11T12:35:01Z" },
        { "status": "question", "timestamp": "2026-02-11T12:35:03Z" },
        { "status": "working", "timestamp": "2026-02-11T12:35:05Z" },
        { "status": "closed", "timestamp": "2026-02-11T12:35:10Z" }
      ]
    }
  ]
}
```

The session should appear with `"status": "closed"` and a history showing all
status transitions.

### Step 7: Test TUI (Manual)

Launch the terminal UI to verify sessions are visible.

```sh
acd tui
```

Expected behavior:

- TUI displays a table with session list
- Session `test-session-001` appears in the list
- Status column shows "Closed"
- Working directory column shows "/tmp/test-project"
- No errors or panics

Press `q` to exit the TUI.

### Step 8: Stop Daemon

Gracefully stop the daemon.

```sh
acd daemon-stop
```

Expected output:

```text
Daemon stopped.
```

If the daemon has active sessions, it will prompt for confirmation:

```text
Warning: 1 active session(s) are running.
Stopping the daemon will disconnect the TUI but Claude Code sessions will continue.
Stop daemon anyway? (y/N):
```

Type `y` to confirm.

Verify daemon is stopped:

```sh
acd status
```

Expected output:

```text
Agent Console Daemon
  Status:      not running
```

### Step 9: Uninstall Plugin

Remove ACD hooks from Claude Code settings.

```sh
acd uninstall
```

Expected output:

```text
  Removed: SessionStart -> acd claude-hook attention
  Removed: UserPromptSubmit -> acd claude-hook working
  Removed: Stop -> acd claude-hook attention
  Removed: SessionEnd -> acd claude-hook closed
  Removed: Notification -> acd claude-hook question
  Removed: Notification -> acd claude-hook attention

Hooks: 6 removed, 0 not managed, 0 errors

Restart Claude Code for changes to take effect.
```

Verify hooks are removed:

```sh
cat ~/.claude/settings.json
```

The `"hooks"` section should be empty or missing.

## Known Issues

### env\_\_fetch_usage_raw panics without token

The daemon may panic if `CLAUDE_CODE_OAUTH_TOKEN` is not set in the environment.
This is expected behavior when the usage tracking feature is enabled but
credentials are unavailable.

Workaround: disable usage tracking or provide credentials.

### Hook JSON schema requirements

Hook stdin must include the following fields:

- `session_id` (string, UUID v4 format recommended)
- `cwd` (string, absolute path to working directory)
- `transcript_path` (string, path to transcript file)
- `permission_mode` (string, e.g., "default")
- `hook_event_name` (string, e.g., "PostToolUse")

Missing `session_id` or `cwd` will cause the hook to fail.

## Success Criteria

All steps should complete without errors. The following must hold:

1. `acd` binary is installed and in PATH
2. Hooks are installed to `~/.claude/settings.json`
3. Daemon starts and responds to `acd status`
4. Simulated hook calls create sessions visible in `acd dump`
5. TUI launches and displays session list
6. Daemon stops gracefully via `acd daemon-stop`
7. Uninstall removes all hooks from settings

## Automation

See `scripts/smoke-test.sh` for an automated version of steps 1-6 and 8-9. The
TUI (step 7) cannot be automated and must be tested manually.
