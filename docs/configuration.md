# Configuration Reference

Agent Console Dashboard is configured via a TOML file located at:

```text
$XDG_CONFIG_HOME/agent-console-dashboard/config.toml
```

If `XDG_CONFIG_HOME` is not set, the default is `~/.config/`.

## Quick Start

```bash
# Create default config file
acd config init

# View current config path
acd config path

# Show effective configuration (defaults + file overrides)
acd config show

# Validate config syntax
acd config validate
```

## Configuration Keys

All configuration keys are optional. If not specified, built-in defaults are
used.

### `[tui]` - Terminal UI Settings

#### `tui.layout`

**Type:** string **Default:** `"default"` **Options:** `"default"`, `"compact"`
**Hot-reloadable:** Yes

Layout preset for the dashboard.

- `default` - Full-featured layout with all panels
- `compact` - Reduced-height layout for smaller terminals

```toml
[tui]
layout = "compact"
```

#### `tui.widgets`

**Type:** array of strings **Default:**
`["session-status:two-line", "api-usage"]` **Hot-reloadable:** Yes

Ordered list of widgets to display. Available widgets:

- `"session-status:two-line"` - Two-line session status display
- `"session-status:one-line"` - Compact one-line session status
- `"api-usage"` - API usage widget

```toml
[tui]
widgets = ["session-status:one-line"]
```

#### `tui.tick_rate`

**Type:** duration string **Default:** `"250ms"` **Hot-reloadable:** No (restart
required)

Controls how often the TUI redraws. Lower values provide smoother updates but
use more CPU.

```toml
[tui]
tick_rate = "500ms"
```

#### `tui.double_click_hook`

**Type:** string **Default:** `""` (disabled) **Hot-reloadable:** Yes

Shell command executed when double-clicking a session. Supports placeholders:

- `{session_id}` - Session's unique identifier
- `{working_dir}` - Session's working directory
- `{status}` - Session's current status (working, attention, question, closed)

Command is executed via `sh -c` in fire-and-forget mode (no callback).

```toml
[tui]
double_click_hook = "code {working_dir}"
```

### `[agents.claude-code]` - Claude Code Integration

#### `agents.claude-code.enabled`

**Type:** boolean **Default:** `true` **Hot-reloadable:** No (restart required)

Enable or disable Claude Code session tracking entirely.

```toml
[agents.claude-code]
enabled = false
```

#### `agents.claude-code.hooks_path`

**Type:** string **Default:** `"~/.claude/hooks"` **Hot-reloadable:** No
(restart required)

Path to Claude Code hooks directory. Tilde (`~`) is expanded to home directory.

```toml
[agents.claude-code]
hooks_path = "/custom/path/hooks"
```

### `[integrations.zellij]` - Zellij Terminal Multiplexer

#### `integrations.zellij.enabled`

**Type:** boolean **Default:** `true` **Hot-reloadable:** No (restart required)

Enable Zellij terminal multiplexer integration for session resurrection.

```toml
[integrations.zellij]
enabled = false
```

### `[daemon]` - Daemon Process Settings

#### `daemon.idle_timeout`

**Type:** duration string **Default:** `"60m"` **Hot-reloadable:** Yes

Duration of inactivity (no active sessions) before daemon auto-stops. Sessions
with status `closed` or older than 5 minutes count as inactive.

Valid duration formats:

- `"30m"` - 30 minutes
- `"2h"` - 2 hours
- `"90m"` - 90 minutes

```toml
[daemon]
idle_timeout = "2h"
```

#### `daemon.usage_fetch_interval`

**Type:** duration string **Default:** `"3m"` **Hot-reloadable:** Yes

Interval between API usage data fetches. Lower values provide fresher data but
increase API calls.

```toml
[daemon]
usage_fetch_interval = "5m"
```

#### `daemon.log_level`

**Type:** string **Default:** `"info"` **Options:** `"error"`, `"warn"`,
`"info"`, `"debug"`, `"trace"` **Hot-reloadable:** Yes

Logging verbosity level:

- `error` - Only errors
- `warn` - Errors and warnings
- `info` - General operational information (recommended)
- `debug` - Detailed debugging information
- `trace` - Very verbose, includes all internal operations

```toml
[daemon]
log_level = "debug"
```

#### `daemon.log_file`

**Type:** string **Default:** `""` (stderr) **Hot-reloadable:** No (restart
required)

Path to log file. Empty string means log to stderr.

```toml
[daemon]
log_file = "/var/log/agent-console-dashboard.log"
```

## Duration Format

Duration fields accept human-readable strings parsed by the `humantime` crate:

- `"250ms"` - 250 milliseconds
- `"3s"` - 3 seconds
- `"5m"` - 5 minutes
- `"2h"` - 2 hours
- `"1d"` - 1 day

Units can be combined: `"1h30m"` (1 hour 30 minutes)

## Hot-Reloadable Settings

Settings marked as hot-reloadable take effect without restarting the daemon or
TUI. Changes are detected on the next config check (typically within seconds).

Non-hot-reloadable settings require:

- Daemon restart for daemon-specific settings
- TUI restart for TUI-specific settings

## Example Configuration

```toml
# Minimal custom configuration
[tui]
layout = "compact"
double_click_hook = "code {working_dir}"

[daemon]
idle_timeout = "2h"
log_level = "debug"
```

## Troubleshooting

### Configuration not taking effect

1. Verify config syntax: `acd config validate`
2. Check loaded config: `acd config show`
3. Ensure config is at correct path: `acd config path`
4. Restart daemon/TUI if setting is not hot-reloadable

### Invalid duration format

Duration parsing follows `humantime` crate rules. Common errors:

- Missing unit: `"60"` → use `"60s"` or `"60m"`
- Invalid unit: `"60min"` → use `"60m"`
- Typo: `"3sec"` → use `"3s"`

Run `acd config validate` to see specific parse errors.
