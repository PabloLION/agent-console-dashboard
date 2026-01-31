# Claude Code Hooks Integration

## Overview

Agent Console Dashboard uses Claude Code hooks to track session lifecycle in
real time. When Claude Code starts working, stops, or needs attention, hook
scripts notify the dashboard daemon via the `acd` CLI. This lets you monitor
multiple Claude Code sessions from a single TUI dashboard.

Three hooks are provided:

- **Stop** - marks a session as needing attention when Claude Code finishes
- **UserPromptSubmit** - marks a session as working when you send a prompt
- **Notification** - marks a session as needing attention on notifications

## Prerequisites

- Agent Console Dashboard installed (`acd` binary in PATH)
- Claude Code v2.0.76+ (required for full hook support)
- Bash shell
- `jq` command-line JSON processor

Install jq if missing:

```sh
# macOS
brew install jq

# Debian/Ubuntu
sudo apt-get install jq

# RHEL/CentOS
sudo yum install jq
```

## Quick Start

```sh
# Copy hook scripts
mkdir -p ~/.claude/hooks
cp scripts/hooks/*.sh ~/.claude/hooks/
chmod +x ~/.claude/hooks/*.sh

# Add hooks to settings.json (creates file if missing)
cat > /tmp/acd-hooks.json << 'HOOKJSON'
{
  "hooks": {
    "Stop": ["~/.claude/hooks/stop.sh"],
    "UserPromptSubmit": ["~/.claude/hooks/user-prompt-submit.sh"],
    "Notification": ["~/.claude/hooks/notification.sh"]
  }
}
HOOKJSON

# Merge with existing settings or use as-is
if [ -f ~/.claude/settings.json ]; then
  jq -s '.[0] * .[1]' ~/.claude/settings.json /tmp/acd-hooks.json > /tmp/acd-merged.json
  mv /tmp/acd-merged.json ~/.claude/settings.json
else
  mkdir -p ~/.claude
  mv /tmp/acd-hooks.json ~/.claude/settings.json
fi

# Verify
echo '{"session_id":"test","cwd":"/tmp","hook_event_name":"Stop"}' | ~/.claude/hooks/stop.sh
acd list
```

## Detailed Setup

### Copy Hook Scripts

Copy all three hook scripts to `~/.claude/hooks/` and make them executable:

```sh
mkdir -p ~/.claude/hooks

cp scripts/hooks/stop.sh ~/.claude/hooks/stop.sh
cp scripts/hooks/user-prompt-submit.sh ~/.claude/hooks/user-prompt-submit.sh
cp scripts/hooks/notification.sh ~/.claude/hooks/notification.sh

chmod +x ~/.claude/hooks/*.sh
```

### Configure settings.json

Open `~/.claude/settings.json` in your editor and add the `hooks` key. If the
file does not exist, create it with this content:

```json
{
  "hooks": {
    "Stop": ["~/.claude/hooks/stop.sh"],
    "UserPromptSubmit": ["~/.claude/hooks/user-prompt-submit.sh"],
    "Notification": ["~/.claude/hooks/notification.sh"]
  }
}
```

If you already have a settings.json with other configuration, merge the `hooks`
key into the existing object:

```json
{
  "existingSetting": "value",
  "anotherSetting": true,
  "hooks": {
    "Stop": ["~/.claude/hooks/stop.sh"],
    "UserPromptSubmit": ["~/.claude/hooks/user-prompt-submit.sh"],
    "Notification": ["~/.claude/hooks/notification.sh"]
  }
}
```

### Verify Installation

Run the dashboard daemon, then test a hook manually:

```sh
# Start the daemon if not running
acd daemon &

# Send sample JSON to the stop hook
echo '{"session_id":"verify-test","cwd":"/tmp","hook_event_name":"Stop"}' \
  | ~/.claude/hooks/stop.sh

# Check that the session appeared
acd list

# Clean up
acd rm verify-test
```

## Settings.json Reference

### Location

| Platform | Path                      |
| -------- | ------------------------- |
| macOS    | `~/.claude/settings.json` |
| Linux    | `~/.claude/settings.json` |

### Format

The `hooks` key maps event names to arrays of executable script paths:

```json
{
  "hooks": {
    "<EventName>": ["<path/to/script1>", "<path/to/script2>"]
  }
}
```

### Hook Event Names

Hook names are **case-sensitive**. Use exactly these names:

| Event Name         | Fires when                             |
| ------------------ | -------------------------------------- |
| `Stop`             | Claude Code session stops or completes |
| `UserPromptSubmit` | User submits a prompt                  |
| `Notification`     | Claude Code sends a notification       |

Each event accepts an array of script paths. Scripts execute in array order.
Paths support `~` expansion for the home directory. All scripts must be
executable (`chmod +x`).

## Hook JSON Stdin Format

Claude Code passes JSON data via stdin to every hook script. All hooks receive
these common fields:

| Field             | Type   | Description                     |
| ----------------- | ------ | ------------------------------- |
| `session_id`      | string | Unique session identifier       |
| `cwd`             | string | Current working directory       |
| `transcript_path` | string | Path to conversation transcript |
| `permission_mode` | string | Current permission mode         |
| `hook_event_name` | string | Which hook fired (e.g., `Stop`) |

Some events include additional fields (e.g., `prompt` for UserPromptSubmit,
`message` for Notification).

**Critical:** Scripts must parse `session_id` from JSON stdin using `jq`. Do
**not** use `basename "$PWD"` -- that pattern is incorrect and unreliable.

Example extraction in bash:

```sh
INPUT=$(cat)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
```

## Troubleshooting

### Hooks Not Firing

- Validate settings.json is valid JSON: `jq . ~/.claude/settings.json`
- Check hook name spelling -- names are case-sensitive (`Stop`, not `stop`)
- Confirm scripts are executable: `ls -l ~/.claude/hooks/*.sh`
- Verify Claude Code version: `claude --version` (need v2.0.76+)

### Dashboard Not Updating

- Confirm the daemon is running: `acd list`
- Verify `acd` is in PATH: `which acd`
- Test a script manually:
  `echo '{"session_id":"test"}' | ~/.claude/hooks/stop.sh`
- Check jq is installed: `which jq`

### Permission Denied

```sh
chmod +x ~/.claude/hooks/*.sh
```

Also verify file ownership matches your user.

### Wrong Session ID

- Hooks must parse JSON stdin, not use `basename $PWD`
- Verify jq works: `echo '{"session_id":"abc123"}' | jq -r '.session_id'`
- Check the hook script reads from stdin (`INPUT=$(cat)`)

### Multiple Hooks on Same Event

Each event name maps to an array. Add multiple scripts to the array:

```json
{
  "hooks": {
    "Stop": ["~/.claude/hooks/stop.sh", "~/.claude/hooks/my-custom-stop.sh"]
  }
}
```

Check for duplicate entries if behavior seems doubled.

## Verification Commands

```sh
# Confirm acd is accessible
acd list

# Test stop hook with sample JSON
echo '{"session_id":"test-session","cwd":"/tmp","hook_event_name":"Stop"}' \
  | ~/.claude/hooks/stop.sh

# Test user-prompt-submit hook
echo '{"session_id":"test-session","cwd":"/tmp","hook_event_name":"UserPromptSubmit"}' \
  | ~/.claude/hooks/user-prompt-submit.sh

# Test notification hook
echo '{"session_id":"test-session","cwd":"/tmp","hook_event_name":"Notification"}' \
  | ~/.claude/hooks/notification.sh

# View dashboard
acd tui

# Clean up test session
acd rm test-session
```
