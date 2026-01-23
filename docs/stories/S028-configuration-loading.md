# Story: Implement Configuration Loading

**Story ID:** S028 **Epic:**
[E007 - Configuration System](../epic/E007-configuration-system.md) **Status:**
Draft **Priority:** P1 **Estimated Points:** 3

## Description

As a developer, I want configuration to be loaded from the TOML file at startup,
So that user preferences are applied when the daemon or dashboard starts.

## Context

With the configuration schema defined (S027), this story implements the actual
loading mechanism. The configuration loader must handle various scenarios: file
exists with valid config, file exists with invalid config, file doesn't exist
(use defaults), and permission errors. Error messages must be user-friendly,
pointing to the exact line and column of any parsing errors.

The loader integrates with the XDG path resolution (S029) to find the
configuration file, but this story focuses on the loading and parsing logic
itself, with path resolution abstracted through a trait or function parameter.

## Implementation Details

### Technical Approach

1. Create `ConfigLoader` struct with methods for loading configuration
2. Implement file reading with proper error handling
3. Use `toml::from_str()` for parsing with position-aware error reporting
4. Provide `load_from_path()` for explicit path and `load_default()` for
   standard location
5. Return `Result<Config, ConfigError>` with descriptive error types

### Files to Modify

- `src/config/loader.rs` - Create configuration loader implementation
- `src/config/error.rs` - Define configuration error types
- `src/config/mod.rs` - Export loader module
- `src/daemon/main.rs` - Integrate config loading at daemon startup
- `src/tui/main.rs` - Integrate config loading at TUI startup

### Dependencies

- [S027 - TOML Configuration Schema](./S027-toml-configuration-schema.md) -
  Schema types must be defined first
- [S029 - XDG Path Support](./S029-xdg-path-support.md) - For default path
  resolution (can develop in parallel with path as parameter)

## Acceptance Criteria

- [ ] Given a valid config file at the specified path, when `load_from_path()`
      is called, then Config is returned
- [ ] Given an invalid TOML file, when loading, then error includes file path,
      line number, and column
- [ ] Given a non-existent config file, when `load_default()` is called, then
      default Config is returned
- [ ] Given a file with permission errors, when loading, then a descriptive
      error is returned
- [ ] Given daemon startup, when config loading fails with invalid TOML, then
      daemon exits with helpful error message
- [ ] Given TUI startup, when config loading fails, then error is displayed
      before exit
- [ ] Given partial configuration, when loading, then missing fields use schema
      defaults

## Testing Requirements

- [ ] Unit test: Load valid configuration from string
- [ ] Unit test: Load valid configuration from file
- [ ] Unit test: Handle missing file gracefully with defaults
- [ ] Unit test: Parse error includes line and column information
- [ ] Unit test: IO error includes file path
- [ ] Integration test: Daemon starts with valid config file
- [ ] Integration test: Daemon exits gracefully with invalid config file

## Out of Scope

- XDG path resolution implementation (S029)
- Creating default configuration file (S030)
- Configuration validation beyond TOML parsing
- Configuration reloading at runtime
- Environment variable overrides

## Notes

### Error Types

```rust
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read configuration file: {path}")]
    ReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Invalid configuration at {path}:{line}:{column}: {message}")]
    ParseError {
        path: PathBuf,
        line: usize,
        column: usize,
        message: String,
    },

    #[error("Configuration file not found: {path}")]
    NotFound { path: PathBuf },
}
```

### Loader Implementation

```rust
use std::path::Path;
use std::fs;

pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration from a specific path
    pub fn load_from_path(path: &Path) -> Result<Config, ConfigError> {
        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::ReadError {
                path: path.to_path_buf(),
                source: e,
            })?;

        toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError {
                path: path.to_path_buf(),
                line: e.line_col().map(|(l, _)| l).unwrap_or(0),
                column: e.line_col().map(|(_, c)| c).unwrap_or(0),
                message: e.message().to_string(),
            })
    }

    /// Load configuration from default location, returning defaults if not found
    pub fn load_default() -> Result<Config, ConfigError> {
        let path = xdg::config_path(); // From S029

        if path.exists() {
            Self::load_from_path(&path)
        } else {
            Ok(Config::default())
        }
    }

    /// Load configuration, creating default file if none exists
    pub fn load_or_create_default() -> Result<Config, ConfigError> {
        let path = xdg::config_path();

        if path.exists() {
            Self::load_from_path(&path)
        } else {
            let config = Config::default();
            // Optionally create default file (S030)
            Ok(config)
        }
    }
}
```

### Error Message Format

User-friendly error messages are critical for configuration issues:

```text
Error: Invalid configuration at ~/.config/agent-console/config.toml:12:5
  Invalid value for 'layout': expected one of "one-line", "two-line", "detailed", "history"

Hint: Check the configuration documentation at https://...
```

### Integration Points

The configuration should be loaded early in the startup sequence:

```rust
// In daemon main
fn main() -> Result<()> {
    let config = ConfigLoader::load_default()
        .context("Failed to load configuration")?;

    // Use config for daemon initialization
    let daemon = Daemon::new(config)?;
    daemon.run()
}

// In TUI main
fn main() -> Result<()> {
    let config = ConfigLoader::load_default()
        .context("Failed to load configuration")?;

    // Use config for TUI initialization
    let app = App::new(config)?;
    app.run()
}
```

### Logging Configuration Loading

Log configuration loading for debugging:

```rust
tracing::info!("Loading configuration from {:?}", path);
tracing::debug!("Configuration loaded: {:?}", config);
```
