# Decision: Configuration and Hot Reload

**Decided:** 2026-01-17 **Status:** Implemented

## Context

The project needed a configuration strategy covering file location, default
behavior without a config file, hot-reload capability during development, and
environment variable overrides.

## Decision

### Location (Q3)

Configuration lives at `~/.config/agent-console/config.toml` following the XDG
standard. The `directories` crate handles platform-appropriate paths.

| Type   | Variable           | Default           | Use                                   |
| ------ | ------------------ | ----------------- | ------------------------------------- |
| Config | `$XDG_CONFIG_HOME` | `~/.config/`      | `~/.config/agent-console/config.toml` |
| Data   | `$XDG_DATA_HOME`   | `~/.local/share/` | Not needed (volatile state)           |

### First Run (Q65)

The daemon works without any config file. Config is optional, only needed for
customization. No setup wizard or auto-generated config.

### Hot Reload (Q27)

Hot reload is supported in v0 via SIGHUP or `acd reload`.

| Setting              | Hot-reloadable?           |
| -------------------- | ------------------------- |
| Colors               | Yes                       |
| Tick interval        | Yes                       |
| Display mode         | Yes                       |
| Auto-stop thresholds | Yes                       |
| Socket path          | **No** (restart required) |
| Log file location    | **No** (restart required) |

Invalid config is rejected: the daemon keeps the old config and logs an error.

### Override Priority (Q64)

Environment variable > config file > default. All settings are overridable via
env vars (e.g., `ACD_SOCKET_PATH=/tmp/test.sock acd daemon`).

### Unknown Keys (Q63)

Unknown config keys produce a warning but do not prevent startup. Typos get
noticed; old configs still work.

## Defaults (Q39)

| Setting                 | Default | Source |
| ----------------------- | ------- | ------ |
| `auto_stop_idle_secs`   | 3600    | D5     |
| `client_timeout_secs`   | 5       | Q29    |
| `usage_fetch_interval`  | 180     | D4     |
| `history_max_entries`   | 200     | Q38    |
| `history_max_age_hours` | 24      | Q38    |
| `session_soft_limit`    | 50      | Q36    |
| `dashboard_soft_limit`  | 50      | Q37    |

## Rationale

- XDG keeps `$HOME` clean and follows platform conventions
- Hot reload speeds up development iteration and matches standard daemon
  behavior
- Zero-config first run eliminates friction for new users

## Implementation

[Q3, Q27, Q39](../archive/planning/6-open-questions.md) |
[Q63-Q65](../archive/planning/6-open-questions.md)
