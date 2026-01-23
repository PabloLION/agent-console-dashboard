# Epic: Configuration System

**Epic ID:** E007
**Status:** Draft
**Priority:** Medium
**Estimated Effort:** M

## Summary

Implement a centralized configuration system using TOML format that manages all application settings. This includes UI layout preferences, widget configurations, agent integrations, and daemon options. The system follows XDG Base Directory conventions for consistent file placement across Unix systems.

## Goals

- Provide a single configuration file for all application settings
- Support XDG Base Directory Specification for standard config file locations
- Enable customization of UI layouts and widgets
- Allow configuration of agent integrations (Claude Code, future agents)
- Create sensible default configuration for out-of-the-box experience

## User Value

Users can customize their dashboard experience through a single, well-documented configuration file. Instead of scattered settings or command-line flags, all preferences are managed in one place at `~/.config/agent-console/config.toml`. This enables easy backup, sharing of configurations, and consistent behavior across sessions. The sensible defaults mean users can start immediately without mandatory configuration.

## Stories

| Story ID | Title | Priority | Status |
|----------|-------|----------|--------|
| [S027](../stories/S027-toml-configuration-schema.md) | Define TOML configuration schema | P1 | Draft |
| [S028](../stories/S028-configuration-loading.md) | Implement configuration loading | P1 | Draft |
| [S029](../stories/S029-xdg-path-support.md) | Add XDG path support | P2 | Draft |
| [S030](../stories/S030-default-configuration-file.md) | Create default configuration file | P2 | Draft |

## Dependencies

- [E001 - Daemon Core Infrastructure](./E001-daemon-core-infrastructure.md) - Daemon must load configuration on startup
- [E005 - Widget System](./E005-widget-system.md) - Widget configuration options depend on available widgets

## Acceptance Criteria

- [ ] Configuration file uses TOML format at `~/.config/agent-console/config.toml`
- [ ] Application loads and parses configuration on startup without errors
- [ ] XDG_CONFIG_HOME environment variable is respected for config location
- [ ] Missing configuration file creates sensible defaults automatically
- [ ] Invalid configuration produces helpful error messages with line numbers
- [ ] All configurable options are documented in the default config file

## Technical Notes

### Configuration File Location

Following XDG Base Directory Specification:

```text
Primary: $XDG_CONFIG_HOME/agent-console/config.toml
Default: ~/.config/agent-console/config.toml
```

### Configuration Schema

```toml
[ui]
layout = "two-line"  # or "one-line", "custom"
widgets = ["working-dir", "status", "api-usage"]

[agents.claude-code]
enabled = true
hooks_path = "~/.claude/hooks"

[integrations.zellij]
enabled = true
```

### Configuration Sections

| Section | Purpose |
|---------|---------|
| `[ui]` | Dashboard layout and widget preferences |
| `[agents.*]` | Per-agent settings (Claude Code, future agents) |
| `[integrations.*]` | External tool integrations (Zellij, etc.) |
| `[daemon]` | Daemon-specific settings (socket path, etc.) |

### Implementation Approach

Use the `toml` and `serde` crates for parsing:

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    ui: UiConfig,
    agents: AgentsConfig,
    integrations: IntegrationsConfig,
    daemon: Option<DaemonConfig>,
}
```

### Configuration Loading Order

1. Check `$XDG_CONFIG_HOME/agent-console/config.toml`
2. Fall back to `~/.config/agent-console/config.toml`
3. If no file exists, use built-in defaults
4. Optionally create default config file on first run

### Error Handling

Configuration errors should be user-friendly:

```text
Error loading configuration:
  ~/.config/agent-console/config.toml:12:5
  Invalid value for 'layout': expected one of "one-line", "two-line", "custom"
```

### Testing Strategy

- Unit tests for TOML parsing with various valid configurations
- Unit tests for error handling with malformed TOML
- Integration tests for XDG path resolution
- Integration tests for default configuration creation
