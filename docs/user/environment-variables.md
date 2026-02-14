# Environment Variables

Environment variables that affect Agent Console Dashboard behavior.

## Logging

### AGENT_CONSOLE_DASHBOARD_LOG

Controls daemon log verbosity via the `tracing` filter syntax.

```csv
Variable,Default,Example
AGENT_CONSOLE_DASHBOARD_LOG,info,"AGENT_CONSOLE_DASHBOARD_LOG=debug acd daemon start"
```

Accepts standard tracing filter directives: `error`, `warn`, `info`, `debug`,
`trace`.

Can also target specific modules:
`AGENT_CONSOLE_DASHBOARD_LOG=agent_console_dashboard::daemon=debug`.

Output is written to stderr in both foreground and background daemon modes.

**Note:** This variable is read at daemon startup. Changing it has no effect on
an already running daemon. Stop and restart the daemon for log level changes to
take effect.

## Path Resolution

### XDG_CONFIG_HOME

Overrides the default configuration directory on all platforms.

```csv
Platform,Default,With override
macOS,~/Library/Application Support/agent-console-dashboard,$XDG_CONFIG_HOME/agent-console-dashboard
Linux,~/.config/agent-console-dashboard,$XDG_CONFIG_HOME/agent-console-dashboard
```

### XDG_RUNTIME_DIR

Overrides the runtime directory used for Unix domain sockets.

```csv
Platform,Default,With override
macOS,$TMPDIR (per-user secure directory),$XDG_RUNTIME_DIR
Linux,/tmp (usually set by systemd),$XDG_RUNTIME_DIR
```

### TMPDIR

macOS-specific fallback for runtime directory when `XDG_RUNTIME_DIR` is not set.

Typically set by the system to a per-user secure directory like
`/var/folders/xx/.../T/`.

Used only on macOS. Linux always falls back to `/tmp`.

## Authentication

### CLAUDE_CODE_OAUTH_TOKEN

Overrides file-based credential storage with a direct OAuth token.

Takes precedence over platform-specific credential storage (macOS Keychain or
Linux `~/.claude/.credentials.json`).

Empty values are ignored — the variable must contain a non-empty token string.

Used primarily for testing and CI environments where keychain/file access is not
available.

## Build System

### NO_INSTALL_HOOKS

Disables automatic git hook installation during `cargo build`.

```csv
Variable,Default,Effect
NO_INSTALL_HOOKS,(not set),Git hooks installed at build time
NO_INSTALL_HOOKS,1,Skip git hook installation
```

Useful for CI environments or when managing git hooks externally.

Only affects build-time behavior — has no effect on runtime.

## Terminal Environment Detection

The following environment variables are read for terminal multiplexer detection
but are not set or modified by ACD:

### ZELLIJ

Set by Zellij when running inside a Zellij session.

Presence indicates Zellij environment. Used to determine which terminal
integration commands to use.

### TMUX

Set by tmux when running inside a tmux session.

Presence indicates tmux environment. Used to determine which terminal
integration commands to use.

When spawning commands that need to interact with the terminal multiplexer, ACD
temporarily unsets `TMUX` and `ZELLIJ` to prevent nested session issues.
