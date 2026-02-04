//! Error types for claude-hooks
//!
//! This module defines the error hierarchy using thiserror for structured
//! error handling across settings, registry, and hook operations.

use std::path::PathBuf;
use thiserror::Error;

use crate::types::HookEvent;

/// Top-level error type
#[derive(Debug, Error)]
pub enum Error {
    /// Settings file error
    #[error(transparent)]
    Settings(#[from] SettingsError),

    /// Registry error
    #[error(transparent)]
    Registry(#[from] RegistryError),

    /// Hook logic error
    #[error(transparent)]
    Hook(#[from] HookError),
}

/// Settings file errors
#[derive(Debug, Error)]
pub enum SettingsError {
    /// Settings file not found
    #[error("Settings file not found: {0}")]
    NotFound(PathBuf),

    /// I/O error reading settings
    #[error("Failed to read settings: {0}")]
    Io(#[source] std::io::Error),

    /// Failed to parse settings file
    #[error("Failed to parse settings: {0}")]
    Parse(String),

    /// Failed to write settings atomically
    #[error("Failed to write settings atomically: {path} - Safety copy at: {temp_path}")]
    WriteAtomic {
        /// Path to the settings file
        path: PathBuf,
        /// Path to the temporary safety copy
        temp_path: PathBuf,
    },
}

/// Registry errors
#[derive(Debug, Error)]
pub enum RegistryError {
    /// I/O error reading registry
    #[error("Failed to read registry: {0}")]
    Io(#[source] std::io::Error),

    /// Failed to parse registry file
    #[error("Failed to parse registry: {0}")]
    Parse(String),

    /// Failed to write registry
    #[error("Failed to write registry: {0}")]
    Write(String),
}

/// Hook logic errors
#[derive(Debug, Error)]
pub enum HookError {
    /// Hook already exists
    #[error("Hook already exists: {event:?} - {command}")]
    AlreadyExists {
        /// The hook event
        event: HookEvent,
        /// The command string
        command: String,
    },

    /// Hook not managed by claude-hooks
    #[error("Hook not managed by claude-hooks: {event:?} - {command}")]
    NotManaged {
        /// The hook event
        event: HookEvent,
        /// The command string
        command: String,
    },

    /// Invalid hook handler
    #[error("Invalid hook handler: {0}")]
    InvalidHandler(String),
}

/// Result type alias for claude-hooks operations
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_error_not_found_display() {
        let path = PathBuf::from("/home/user/.claude/settings.json");
        let err = SettingsError::NotFound(path);
        let display = format!("{}", err);
        assert!(
            display.contains("/home/user/.claude/settings.json"),
            "Error should contain path"
        );
    }

    #[test]
    fn test_settings_error_write_atomic_display() {
        let path = PathBuf::from("/home/user/.claude/settings.json");
        let temp_path = PathBuf::from("/home/user/.claude/settings.json.tmp.20260203-143022");
        let err = SettingsError::WriteAtomic { path, temp_path };
        let display = format!("{}", err);
        assert!(
            display.contains("/home/user/.claude/settings.json"),
            "Error should contain path"
        );
        assert!(
            display.contains("settings.json.tmp.20260203-143022"),
            "Error should contain temp path"
        );
        assert!(display.contains("Safety copy"), "Error should mention safety copy");
    }

    #[test]
    fn test_hook_error_already_exists_display() {
        let err = HookError::AlreadyExists {
            event: HookEvent::Stop,
            command: "/path/to/stop.sh".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("Stop"), "Error should contain event");
        assert!(
            display.contains("/path/to/stop.sh"),
            "Error should contain command"
        );
    }

    #[test]
    fn test_hook_error_not_managed_display() {
        let err = HookError::NotManaged {
            event: HookEvent::Start,
            command: "/path/to/start.sh".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("Start"), "Error should contain event");
        assert!(
            display.contains("/path/to/start.sh"),
            "Error should contain command"
        );
        assert!(
            display.contains("not managed"),
            "Error should indicate not managed"
        );
    }

    #[test]
    fn test_registry_error_parse_display() {
        let err = RegistryError::Parse("Invalid JSON at line 5".to_string());
        let display = format!("{}", err);
        assert!(
            display.contains("Invalid JSON at line 5"),
            "Error should contain parse details"
        );
    }
}
