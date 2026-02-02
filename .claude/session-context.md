# Session Context

## Goal

Wire end-to-end IPC and plan `claude-hook-install` standalone crate.

## Progress

### Completed and committed

- `2e981f9` feat: wire end-to-end IPC between daemon, CLI, and TUI
  - `daemon/mod.rs`: SocketServer now starts inside `run_daemon()` with shutdown
    support
  - `main.rs`: Added `acd set <session_id> <status>` CLI command for hooks
  - `tui/app.rs`: TUI connects to daemon via `connect_with_auto_start()`,
    fetches LIST, subscribes via SUB, renders live updates
- Data flow now works: hook fires → `acd set` → daemon socket → SessionStore →
  broadcast → TUI

### Researched but not started

- `claude-hook-install` standalone crate — user wants a separate package (like
  `claude-usage`) that installs hooks into Claude Code's
  `~/.claude/settings.json`

## What We Have (hooks)

- `scripts/hooks/stop.sh` — sets session to "attention" on Stop event
- `scripts/hooks/user-prompt-submit.sh` — sets session to "working" on
  UserPromptSubmit
- `scripts/hooks/notification.sh` — sets session to "attention" on Notification
- `docs/integration/claude-code-hooks.md` — manual installation docs
- All hooks: read JSON stdin, extract `session_id` via `jq`, call `acd set`

## What We Need (`claude-hook-install` crate)

### Purpose

Standalone CLI tool to install/uninstall command hooks into Claude Code's
settings.json. Similar pattern to `claude-usage` crate — independent but used by
agent-console-dashboard.

### Claude Code hook registration format

Hooks live in `~/.claude/settings.json` (global) or `.claude/settings.json`
(project):

```json
{
  "hooks": {
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          { "type": "command", "command": "/path/to/stop.sh", "timeout": 600 }
        ]
      }
    ],
    "UserPromptSubmit": [
      { "hooks": [{ "type": "command", "command": "/path/to/script.sh" }] }
    ],
    "Notification": [
      { "hooks": [{ "type": "command", "command": "/path/to/script.sh" }] }
    ]
  }
}
```

### Hook events available

SessionStart, UserPromptSubmit, PreToolUse, PermissionRequest, PostToolUse,
PostToolUseFailure, Notification, SubagentStart, SubagentStop, Stop, PreCompact,
SessionEnd

### Customizable fields per hook handler

- `type`: "command" | "prompt" | "agent"
- `command`: shell command to run (for type=command)
- `prompt`: LLM prompt (for type=prompt/agent)
- `model`: model override (for prompt/agent)
- `timeout`: seconds (default 600 for command, 30 for prompt, 60 for agent)
- `async`: bool (background execution, command only)
- `statusMessage`: custom spinner text
- `once`: bool (run once per session, skills only)

### Matcher per event

- Tool events (PreToolUse, PostToolUse, etc.): regex on tool name
- SessionStart: startup|resume|clear|compact
- SessionEnd: clear|logout|prompt_input_exit|other
- Notification: permission_prompt|idle_prompt|auth_success|elicitation_dialog
- SubagentStart/Stop: agent type name
- PreCompact: manual|auto
- UserPromptSubmit, Stop: no matcher (always fires)

### Scope levels

1. `~/.claude/settings.json` — global (all projects)
2. `.claude/settings.json` — project (committable)
3. `.claude/settings.local.json` — project local (gitignored)

### Crate API sketch

```rust
// crate: claude-hook-install (or claude-hooks)
// Located at: crates/claude-hook-install/

struct HookConfig { event: HookEvent, matcher: Option<String>, handler: HookHandler }
enum HookEvent { Stop, UserPromptSubmit, Notification, PreToolUse, ... }
enum HookHandler { Command { command: String, timeout: Option<u64>, async_: bool }, ... }
enum Scope { Global, Project, ProjectLocal }

fn install_hooks(scope: Scope, hooks: &[HookConfig]) -> Result<()>
fn uninstall_hooks(scope: Scope, hooks: &[HookConfig]) -> Result<()>
fn list_hooks(scope: Scope) -> Result<Vec<HookConfig>>
```

### Integration with agent-console-dashboard

`acd hooks install` would call the library to register our 3 hooks globally.

## Files

- `crates/agent-console-dashboard/src/daemon/mod.rs` — daemon wiring (done)
- `crates/agent-console-dashboard/src/main.rs` — `acd set` command (done)
- `crates/agent-console-dashboard/src/tui/app.rs` — TUI subscription (done)
- `scripts/hooks/{stop,user-prompt-submit,notification}.sh` — hook scripts
  (exist)
- `crates/claude-hook-install/` — new crate (TODO)

## Pending

1. Create `crates/claude-hook-install/` standalone crate
2. Implement settings.json read/merge/write logic
3. Define hook config types and CLI
4. Wire into `acd hooks install/uninstall/status` subcommands
5. User mentioned `~/base/manifold/llm-agents/` has related info (check at
   session start)
