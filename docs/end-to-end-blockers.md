# End-to-End Blockers

What prevents ACD from working as a complete system today. Each blocker
includes the problem, evidence, and a concrete fix.

## Current Hook Coverage

ACD registers 3 hooks via `.claude-plugin/plugin.json` (build artifact):

| Hook              | Status mapped | When it fires                    |
|-------------------|---------------|----------------------------------|
| `SessionStart`    | `attention`   | Session begins or resumes        |
| `UserPromptSubmit`| `working`     | User sends a prompt              |
| `Stop`            | `attention`   | Claude finishes responding       |

Claude Code provides 14 hook events total. Missing events that matter:

| Hook                             | Should map to | Why it matters                       |
|----------------------------------|---------------|--------------------------------------|
| `SessionEnd`                     | `closed`      | Sessions never close otherwise       |
| `Notification` (`elicitation_dialog`)   | `question`    | Detects AskUserQuestion dialogs      |
| `Notification` (`permission_prompt`)    | `attention`   | Detects permission prompts           |

Source: https://code.claude.com/docs/en/hooks

## Blockers

### B1 (P0): Plugin not installed in Claude Code

**Problem:** `.claude-plugin/` is a gitignored build artifact at the workspace
root. Claude Code does not auto-discover it. The plugin must be explicitly
installed or hooks must be added to settings.

**Evidence:** The hooks reference lists these hook locations:

| Location                         | Scope           | Shareable |
|----------------------------------|-----------------|-----------|
| `~/.claude/settings.json`        | All projects    | No        |
| `.claude/settings.json`          | Single project  | Yes       |
| `.claude/settings.local.json`    | Single project  | No        |
| Plugin `hooks/hooks.json`        | When enabled    | Yes       |

Plugins require installation via `claude plugin install <name>` from a
marketplace, or per-session loading with `claude --plugin-dir <path>`.

**Fix options (pick one):**

1. **Global settings (simplest for personal use):** Write hooks directly to
   `~/.claude/settings.json`. Works across all projects immediately.
   An `acd install` command could automate this.

2. **Plugin marketplace (for distribution):** Publish the plugin so others can
   `claude plugin install agent-console-dashboard --scope user`. Requires a
   marketplace entry (local or remote).

3. **Per-session flag:** `claude --plugin-dir /path/to/acd`. Not practical for
   daily use.

**Recommendation:** Option 1 for now. Option 2 when distributing to others.

Source: https://code.claude.com/docs/en/plugins-reference#plugin-installation-scopes

### B2 (P0): `acd` binary not in PATH

**Problem:** Hook commands use bare `acd` (e.g., `acd claude-hook attention`).
After `cargo build`, the binary is at `target/debug/acd` or
`target/release/acd`, not in `$PATH`. Hooks fail silently.

**Evidence:** The hook command in plugin.json:
```json
{ "command": "acd claude-hook attention" }
```

Claude Code runs this as a shell command. If `acd` is not found, the hook
exits non-zero and Claude Code ignores the failure (non-blocking).

**Fix:** `cargo install --path crates/agent-console-dashboard` puts the binary
at `~/.cargo/bin/acd`. An `acd install` command should verify this.

### B3 (P1): Missing hook events

**Problem:** Only 3 of 14 hook events are registered. Key gaps:

- **No `SessionEnd` hook:** Sessions are never marked `closed` by hooks.
  The daemon tracks sessions as active forever until manually closed or
  the daemon restarts.

- **No `Notification` hooks:** `elicitation_dialog` fires when Claude uses
  AskUserQuestion — this should set status to `question`. `permission_prompt`
  fires when a permission dialog appears — this should set `attention`.

- **`Stop` maps to `attention` but should map contextually:** When Claude
  finishes a response normally (waiting for user input), `attention` is
  correct. But the current mapping doesn't distinguish from error states.

**Fix:** Add these hooks to the configuration:

```json
{
  "SessionEnd": [
    {
      "hooks": [{ "type": "command", "command": "acd claude-hook closed" }]
    }
  ],
  "Notification": [
    {
      "matcher": "elicitation_dialog",
      "hooks": [{ "type": "command", "command": "acd claude-hook question" }]
    },
    {
      "matcher": "permission_prompt",
      "hooks": [{ "type": "command", "command": "acd claude-hook attention" }]
    }
  ]
}
```

**Prerequisite:** The `Status` enum currently has `Working`, `Attention`,
`Question`, `Closed`. The `claude-hook` subcommand's `Status` argument must
accept `closed` and `question` as valid values.

Source: https://code.claude.com/docs/en/hooks#hook-events

### B4 (P1): Hook format may need restructuring

**Problem:** Our `plugin.json` embeds hooks directly. The plugin reference says
hooks can be inline in `plugin.json` OR in a separate `hooks/hooks.json`. Both
formats are valid per the docs:

> `hooks`: string|array|object — Hook config paths or inline config

The inline format we use is valid. However, if we switch to `~/.claude/settings.json`
(B1 fix option 1), we bypass the plugin entirely and define hooks directly in
settings. The plugin.json hooks become irrelevant for personal use.

**Decision needed:** Keep plugin.json for distribution AND add `acd install`
for personal setup? Or remove the plugin approach entirely?

### B5 (P2): No root README

**Problem:** No documentation for new users. No explanation of what ACD does,
how to install, or how to use it.

**Fix:** Create `README.md` with: overview, installation, setup, usage.

### B6 (P2): No `acd install` command

**Problem:** No automated setup. Users must manually:
1. Build and install the binary
2. Edit `~/.claude/settings.json` to add hooks
3. Restart Claude Code

**Fix:** Add `acd install` subcommand that:
1. Verifies `acd` is in PATH (or suggests `cargo install`)
2. Writes hooks to `~/.claude/settings.json` (merging with existing hooks)
3. Prints next steps (restart Claude Code)

Corresponding `acd uninstall` to remove hooks cleanly.

### B7 (P2): Socket path not configurable via hooks

**Problem:** The default socket `/tmp/agent-console-dashboard.sock` is
hardcoded in the CLI default. Hook commands don't pass `--socket`, so they
always use the default. This works for single-user setups but won't scale
to multi-user or custom configurations.

**Fix:** Low priority. Default works for personal use.

## End-to-End User Journey (Target)

```
1. cargo install --path crates/agent-console-dashboard
   (puts `acd` in ~/.cargo/bin/)

2. acd install
   (writes hooks to ~/.claude/settings.json, verifies PATH)

3. Restart Claude Code (or start new session)

4. Claude Code fires hooks automatically:
   SessionStart    → acd claude-hook attention  → daemon lazy-starts
   UserPromptSubmit → acd claude-hook working
   Stop            → acd claude-hook attention
   Notification    → acd claude-hook question/attention
   SessionEnd      → acd claude-hook closed

5. acd tui
   (shows live dashboard of all sessions)

6. acd uninstall
   (removes hooks from ~/.claude/settings.json)
```

## What Already Works

- Daemon lifecycle (spawn, socket, sessions, shutdown)
- IPC protocol (SET, STATUS, DUMP, RESURRECT, LIST, SUB)
- TUI rendering (sessions, status, timestamps, usage)
- Lazy-start (daemon auto-spawns from hooks or TUI)
- Config system (TOML from XDG paths)
- All CLI subcommands parse and execute correctly
- Inactive session detection (1 hour threshold)
- Build system generates plugin.json with version sync
