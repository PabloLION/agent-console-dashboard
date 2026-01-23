# Story: Create Default Configuration File

**Story ID:** S030
**Epic:** [E007 - Configuration System](../epic/E007-configuration-system.md)
**Status:** Draft
**Priority:** P2
**Estimated Points:** 2

## Description

As a user,
I want a default configuration file to be created on first run,
So that I have a documented starting point for customizing my dashboard settings.

## Context

When users first install the Agent Console Dashboard, there is no configuration file. While the application works with built-in defaults, having an actual configuration file provides several benefits: users can see all available options, understand the configuration format, and easily customize settings. This story implements the creation of a well-documented default configuration file on first run.

The default configuration file should be heavily commented, explaining each option, its valid values, and the default behavior. It serves as both configuration and documentation.

## Implementation Details

### Technical Approach

1. Create a template default configuration as a Rust const string
2. On first run (config file doesn't exist), write the default file
3. Include comprehensive comments explaining each section and option
4. Set appropriate file permissions (0600)
5. Provide a CLI command to regenerate the default config

### Files to Modify

- `src/config/default.rs` - Create default configuration template
- `src/config/loader.rs` - Add logic to create default file on first run
- `src/config/mod.rs` - Export default config functionality
- `src/cli/commands.rs` - Add `config init` or `config reset` command

### Dependencies

- [S027 - TOML Configuration Schema](./S027-toml-configuration-schema.md) - Schema defines what goes in the config
- [S028 - Configuration Loading](./S028-configuration-loading.md) - Loader determines when to create default
- [S029 - XDG Path Support](./S029-xdg-path-support.md) - Determines where to create the file

## Acceptance Criteria

- [ ] Given no config file exists, when daemon starts for first time, then default config file is created
- [ ] Given the created default config, when read by user, then all options are documented with comments
- [ ] Given the created default config, when parsed, then it equals the built-in defaults
- [ ] Given the config file already exists, when daemon starts, then file is not overwritten
- [ ] Given user runs `agent-console config init`, when config doesn't exist, then default file is created
- [ ] Given user runs `agent-console config init --force`, when config exists, then file is overwritten with backup
- [ ] Given the default config file, when permissions are checked, then it is readable only by owner (0600)

## Testing Requirements

- [ ] Unit test: Default config template parses to valid Config
- [ ] Unit test: Default config template contains all documented sections
- [ ] Integration test: First run creates config file at correct location
- [ ] Integration test: Subsequent runs don't modify existing config
- [ ] Integration test: `config init` command creates file
- [ ] Integration test: `config init --force` creates backup and overwrites

## Out of Scope

- Configuration migration from old versions
- Interactive configuration wizard
- GUI configuration editor
- Configuration validation beyond parsing

## Notes

### Default Configuration Template

```rust
pub const DEFAULT_CONFIG: &str = r#"# Agent Console Dashboard Configuration
#
# This is the default configuration file. All values shown are the defaults.
# Uncomment and modify options to customize your dashboard.
#
# Documentation: https://github.com/user/agent-console-dashboard
# Configuration reference: https://github.com/user/agent-console-dashboard/docs/configuration.md

# ==============================================================================
# UI Configuration
# ==============================================================================

[ui]

# Dashboard layout preset
# Options: "one-line", "two-line", "detailed", "history"
# - one-line: Compact single-line per session (minimal info)
# - two-line: Two lines per session (status + working directory)
# - detailed: Full session details with API usage
# - history: Includes state transition history
layout = "two-line"

# Widgets to display in the dashboard
# Available widgets:
# - "session-status": Shows current status (Working/Attention/Question)
# - "working-dir": Shows the session's working directory
# - "api-usage": Shows token consumption metrics
# - "state-history": Shows recent state transitions
# - "clock": Shows current time
# - "spacer": Adds flexible spacing between widgets
widgets = ["session-status", "working-dir", "api-usage"]

# Color scheme for the dashboard
# Options: "dark", "light", "auto"
# - auto: Follows terminal color scheme (if detectable)
color_scheme = "dark"

# ==============================================================================
# Agent Configuration
# ==============================================================================

[agents.claude-code]

# Enable Claude Code integration
# Set to false to disable Claude Code session tracking
enabled = true

# Path to Claude Code hooks directory
# Hooks scripts should be placed here and registered in Claude Code settings
hooks_path = "~/.claude/hooks"

# ==============================================================================
# Integration Configuration
# ==============================================================================

[integrations.zellij]

# Enable Zellij integration for session resurrection
# When enabled, resurrecting a session can open a new Zellij pane
enabled = true

# ==============================================================================
# Daemon Configuration
# ==============================================================================

[daemon]

# Unix socket filename (relative to $XDG_RUNTIME_DIR or /tmp)
# The full path will be: $XDG_RUNTIME_DIR/agent-console.sock
# or /tmp/agent-console.sock if XDG_RUNTIME_DIR is not set
socket_path = "agent-console.sock"

# Logging verbosity level
# Options: "error", "warn", "info", "debug", "trace"
# - error: Only errors
# - warn: Errors and warnings
# - info: General operational information (recommended)
# - debug: Detailed debugging information
# - trace: Very verbose, includes all internal operations
log_level = "info"
"#;
```

### Creating Default Configuration

```rust
use std::fs;
use std::io::Write;

/// Create the default configuration file if it doesn't exist
pub fn create_default_config_if_missing() -> Result<bool, ConfigError> {
    let path = xdg::config_path();

    if path.exists() {
        return Ok(false); // Already exists, didn't create
    }

    // Ensure the config directory exists
    xdg::ensure_config_dir()?;

    // Write the default configuration
    let mut file = fs::File::create(&path)?;
    file.write_all(DEFAULT_CONFIG.as_bytes())?;

    // Set file permissions to 0600 (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }

    tracing::info!("Created default configuration at {:?}", path);
    Ok(true) // Created new file
}

/// Create default configuration, backing up existing if --force
pub fn create_default_config(force: bool) -> Result<(), ConfigError> {
    let path = xdg::config_path();

    if path.exists() && !force {
        return Err(ConfigError::AlreadyExists { path });
    }

    if path.exists() && force {
        // Create backup
        let backup_path = path.with_extension("toml.backup");
        fs::rename(&path, &backup_path)?;
        tracing::info!("Backed up existing configuration to {:?}", backup_path);
    }

    xdg::ensure_config_dir()?;

    let mut file = fs::File::create(&path)?;
    file.write_all(DEFAULT_CONFIG.as_bytes())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }

    println!("Created configuration file at {}", path.display());
    Ok(())
}
```

### CLI Command

```rust
/// Configuration management commands
#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Create default configuration file
    Init {
        /// Overwrite existing configuration (creates backup)
        #[arg(long)]
        force: bool,
    },
    /// Show current configuration file path
    Path,
    /// Validate configuration file
    Validate,
}

impl ConfigCommand {
    pub fn run(&self) -> Result<()> {
        match self {
            ConfigCommand::Init { force } => {
                create_default_config(*force)?;
            }
            ConfigCommand::Path => {
                println!("{}", xdg::config_path().display());
            }
            ConfigCommand::Validate => {
                let config = ConfigLoader::load_default()?;
                println!("Configuration is valid");
                println!("{:#?}", config);
            }
        }
        Ok(())
    }
}
```

### Behavior on First Run

```
$ agent-console
Created configuration file at /home/user/.config/agent-console/config.toml
Starting daemon...
```

The message is printed to stderr so it doesn't interfere with normal output.

### Documentation Sync

The default configuration comments should be kept in sync with:
- Schema changes (S027)
- Feature documentation
- README.md configuration section

Consider generating parts of the documentation from the default config template.
