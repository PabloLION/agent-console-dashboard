//! Configuration file loader with position-aware error reporting.
//!
//! Loads TOML configuration from a specific path or the default XDG location.
//! When the default location has no file, returns `Config::default()`.

use std::fs;
use std::path::Path;

use crate::config::error::ConfigError;
use crate::config::schema::Config;
use crate::config::xdg;

/// Stateless configuration loader.
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration from a specific path.
    ///
    /// Returns `ConfigError::NotFound` if the file does not exist, or
    /// `ConfigError::ReadError` for other I/O failures.
    pub fn load_from_path(path: &Path) -> Result<Config, ConfigError> {
        let content = fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ConfigError::NotFound {
                    path: path.to_path_buf(),
                }
            } else {
                ConfigError::ReadError {
                    path: path.to_path_buf(),
                    source: e,
                }
            }
        })?;
        Self::parse_toml(&content, path)
    }

    /// Load configuration from the default XDG location.
    ///
    /// If no file exists at the default path, returns `Config::default()`
    /// instead of an error.
    pub fn load_default() -> Result<Config, ConfigError> {
        let path = xdg::config_path();
        if path.exists() {
            Self::load_from_path(&path)
        } else {
            tracing::debug!("No config file at {:?}, using defaults", path);
            Ok(Config::default())
        }
    }

    /// Parse a TOML string into `Config` with position-aware error reporting.
    fn parse_toml(content: &str, path: &Path) -> Result<Config, ConfigError> {
        toml::from_str(content).map_err(|e| {
            let (line, column) = e
                .span()
                .map(|span| {
                    let line = content[..span.start].matches('\n').count() + 1;
                    let last_newline = content[..span.start]
                        .rfind('\n')
                        .map(|p| p + 1)
                        .unwrap_or(0);
                    let column = span.start - last_newline + 1;
                    (line, column)
                })
                .unwrap_or((0, 0));
            ConfigError::ParseError {
                path: path.to_path_buf(),
                line,
                column,
                message: e.message().to_string(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Mutex;

    /// Serialize tests that mutate environment variables.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    /// Run a closure with `XDG_CONFIG_HOME` temporarily set, then restore.
    fn with_xdg_config<F: FnOnce()>(value: Option<&str>, f: F) {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let original = std::env::var("XDG_CONFIG_HOME").ok();
        match value {
            Some(v) => unsafe { std::env::set_var("XDG_CONFIG_HOME", v) },
            None => unsafe { std::env::remove_var("XDG_CONFIG_HOME") },
        }
        f();
        match original {
            Some(v) => unsafe { std::env::set_var("XDG_CONFIG_HOME", v) },
            None => unsafe { std::env::remove_var("XDG_CONFIG_HOME") },
        }
    }

    // -----------------------------------------------------------------------
    // parse_toml
    // -----------------------------------------------------------------------

    #[test]
    fn parse_valid_full_config() {
        let toml_str = r#"
[tui]
layout = "compact"
widgets = ["session-status:one-line"]
tick_rate = "100ms"

[agents.claude-code]
enabled = false
hooks_path = "/custom/hooks"

[integrations.zellij]
enabled = false

[daemon]
idle_timeout = "30m"
usage_fetch_interval = "5m"
log_level = "debug"
log_file = "/var/log/acd.log"
"#;
        let path = PathBuf::from("test.toml");
        let config =
            ConfigLoader::parse_toml(toml_str, &path).expect("valid TOML should parse");
        assert!(!config.agents.claude_code.enabled);
        assert_eq!(config.daemon.idle_timeout, "30m");
    }

    #[test]
    fn parse_empty_string_returns_defaults() {
        let path = PathBuf::from("empty.toml");
        let config =
            ConfigLoader::parse_toml("", &path).expect("empty string should parse to defaults");
        assert_eq!(config, Config::default());
    }

    #[test]
    fn parse_partial_config_fills_defaults() {
        let toml_str = r#"
[daemon]
log_level = "debug"
"#;
        let path = PathBuf::from("partial.toml");
        let config =
            ConfigLoader::parse_toml(toml_str, &path).expect("partial config should parse");
        assert_eq!(
            config.daemon.log_level,
            crate::config::schema::LogLevel::Debug
        );
        assert_eq!(config.daemon.idle_timeout, "60m");
        assert_eq!(config.tui.tick_rate, "250ms");
    }

    #[test]
    fn parse_invalid_toml_returns_parse_error_with_position() {
        let toml_str = "key = \ninvalid";
        let path = PathBuf::from("bad.toml");
        let err = ConfigLoader::parse_toml(toml_str, &path).expect_err("should fail");
        match err {
            ConfigError::ParseError {
                path: p,
                line,
                column,
                message,
            } => {
                assert_eq!(p, path);
                assert!(line > 0, "line should be > 0 for known span");
                assert!(column > 0, "column should be > 0 for known span");
                assert!(!message.is_empty(), "message should not be empty");
            }
            other => panic!("expected ParseError, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // load_from_path
    // -----------------------------------------------------------------------

    #[test]
    fn load_from_path_valid_file() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let file = dir.path().join("config.toml");
        fs::write(&file, "[daemon]\nlog_level = \"trace\"\n")
            .expect("failed to write temp file");
        let config = ConfigLoader::load_from_path(&file).expect("should load");
        assert_eq!(
            config.daemon.log_level,
            crate::config::schema::LogLevel::Trace
        );
    }

    #[test]
    fn load_from_path_missing_file_returns_not_found() {
        let path = PathBuf::from("/tmp/nonexistent_acd_test_config.toml");
        let err = ConfigLoader::load_from_path(&path).expect_err("should fail");
        match err {
            ConfigError::NotFound { path: p } => {
                assert_eq!(p, path);
            }
            other => panic!("expected NotFound, got: {other:?}"),
        }
    }

    #[test]
    fn load_from_path_directory_returns_read_error() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let err = ConfigLoader::load_from_path(dir.path()).expect_err("should fail");
        match err {
            ConfigError::ReadError { path, .. } => {
                assert_eq!(path, dir.path());
            }
            other => panic!("expected ReadError, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // load_default
    // -----------------------------------------------------------------------

    #[test]
    fn load_default_with_no_file_returns_defaults() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        with_xdg_config(Some(dir.path().to_str().expect("non-utf8 path")), || {
            let config = ConfigLoader::load_default().expect("should return defaults");
            assert_eq!(config, Config::default());
        });
    }

    #[test]
    fn load_default_with_existing_file_parses_it() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let config_dir = dir.path().join("agent-console");
        fs::create_dir_all(&config_dir).expect("failed to create config dir");
        fs::write(
            config_dir.join("config.toml"),
            "[daemon]\nlog_level = \"warn\"\n",
        )
        .expect("failed to write config");
        with_xdg_config(Some(dir.path().to_str().expect("non-utf8 path")), || {
            let config = ConfigLoader::load_default().expect("should load");
            assert_eq!(
                config.daemon.log_level,
                crate::config::schema::LogLevel::Warn
            );
        });
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn parse_error_for_wrong_type() {
        let toml_str = "[tui]\ntick_rate = 42\n";
        let path = PathBuf::from("wrong_type.toml");
        let err = ConfigLoader::parse_toml(toml_str, &path).expect_err("should fail");
        matches!(err, ConfigError::ParseError { .. });
    }
}
