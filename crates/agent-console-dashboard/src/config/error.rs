//! Configuration error types for loading and parsing TOML config files.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur when loading or parsing configuration.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Failed to read the configuration file from disk.
    #[error("Failed to read configuration file: {path}")]
    ReadError {
        /// Path to the file that could not be read.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The TOML content could not be parsed.
    #[error("Invalid configuration at {path}:{line}:{column}: {message}")]
    ParseError {
        /// Path to the file containing the error.
        path: PathBuf,
        /// One-based line index of the error (0 if unknown).
        line: usize,
        /// One-based column index of the error (0 if unknown).
        column: usize,
        /// Human-readable description of the parse failure.
        message: String,
    },

    /// An explicitly requested configuration file does not exist.
    #[error("{message}\nPath: {path}")]
    NotFound {
        /// Path that was requested but does not exist.
        path: PathBuf,
        /// Custom error message.
        message: String,
    },

    /// A configuration file already exists at the target path.
    #[error("Configuration file already exists: {path}")]
    AlreadyExists {
        /// Path where the file already exists.
        path: PathBuf,
    },

    /// Failed to write a configuration file to disk.
    #[error("Failed to write configuration file: {path}")]
    WriteError {
        /// Path to the file that could not be written.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to serialize configuration to TOML.
    #[error("Failed to serialize configuration: {message}")]
    SerializeError {
        /// Description of the serialization failure.
        message: String,
    },

    /// No editor is configured ($VISUAL or $EDITOR not set).
    #[error("No editor configured. Set $EDITOR environment variable.")]
    EditorNotSet,

    /// Failed to launch the editor.
    #[error("Failed to launch editor '{editor}'")]
    EditorError {
        /// Editor command that failed to launch.
        editor: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Editor exited with non-zero status.
    #[error("Editor '{editor}' exited with status {}", code.map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string()))]
    EditorFailed {
        /// Editor command that failed.
        editor: String,
        /// Exit code (None if terminated by signal).
        code: Option<i32>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_read_error() {
        let err = ConfigError::ReadError {
            path: PathBuf::from("/etc/app/config.toml"),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied"),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("/etc/app/config.toml"),
            "ReadError display should include the path"
        );
        assert!(
            msg.contains("Failed to read"),
            "ReadError display should describe the failure"
        );
    }

    #[test]
    fn display_parse_error() {
        let err = ConfigError::ParseError {
            path: PathBuf::from("config.toml"),
            line: 5,
            column: 12,
            message: "expected `=`".to_string(),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("5:12"),
            "ParseError should include line:column"
        );
        assert!(
            msg.contains("expected `=`"),
            "ParseError should include the message"
        );
    }

    #[test]
    fn display_not_found_error() {
        let err = ConfigError::NotFound {
            path: PathBuf::from("/missing/config.toml"),
            message: "Configuration file not found".to_string(),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("/missing/config.toml"),
            "NotFound display should include the path"
        );
    }

    #[test]
    fn display_already_exists_error() {
        let err = ConfigError::AlreadyExists {
            path: PathBuf::from("/home/user/.config/agent-console-dashboard/config.toml"),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("already exists"),
            "AlreadyExists display should mention 'already exists'"
        );
        assert!(
            msg.contains("config.toml"),
            "AlreadyExists display should include the path"
        );
    }

    #[test]
    fn display_write_error() {
        let err = ConfigError::WriteError {
            path: PathBuf::from("/tmp/config.toml"),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied"),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("/tmp/config.toml"),
            "WriteError display should include the path"
        );
    }

    #[test]
    fn read_error_source_chain() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        let err = ConfigError::ReadError {
            path: PathBuf::from("/secret"),
            source: io_err,
        };
        // thiserror #[source] makes std::error::Error::source() return Some
        let source = std::error::Error::source(&err);
        assert!(source.is_some(), "ReadError should chain the I/O source");
    }

    #[test]
    fn display_serialize_error() {
        let err = ConfigError::SerializeError {
            message: "invalid TOML structure".to_string(),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("Failed to serialize"),
            "SerializeError display should describe the failure"
        );
        assert!(
            msg.contains("invalid TOML structure"),
            "SerializeError display should include the message"
        );
    }

    #[test]
    fn display_editor_not_set() {
        let err = ConfigError::EditorNotSet;
        let msg = err.to_string();
        assert!(
            msg.contains("EDITOR"),
            "EditorNotSet display should mention EDITOR"
        );
    }

    #[test]
    fn display_editor_error_simple() {
        let err = ConfigError::EditorError {
            editor: "vim".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("vim"),
            "EditorError display should include the editor name"
        );
    }

    #[test]
    fn display_editor_error_with_args_preserves_full_string() {
        // Regression test for acd-ohr6: EDITOR values like 'code-insiders --wait'
        // must appear verbatim in the error message so the user knows what failed.
        let err = ConfigError::EditorError {
            editor: "code-insiders --wait".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("code-insiders --wait"),
            "EditorError display should include the full editor string with arguments"
        );
    }

    #[test]
    fn display_editor_failed_with_exit_code() {
        let err = ConfigError::EditorFailed {
            editor: "vim".to_string(),
            code: Some(1),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("vim"),
            "EditorFailed display should include the editor"
        );
        assert!(
            msg.contains("1"),
            "EditorFailed display should include the exit code"
        );
    }

    #[test]
    fn display_editor_failed_no_exit_code() {
        let err = ConfigError::EditorFailed {
            editor: "vim".to_string(),
            code: None,
        };
        let msg = err.to_string();
        assert!(
            msg.contains("vim"),
            "EditorFailed display should include the editor"
        );
        assert!(
            msg.contains("unknown"),
            "EditorFailed display should say 'unknown' when exit code is None"
        );
    }

    #[test]
    fn display_editor_failed_with_args_preserves_full_string() {
        // Regression test for acd-ohr6: full EDITOR string preserved in failure message.
        let err = ConfigError::EditorFailed {
            editor: "code-insiders --wait".to_string(),
            code: Some(1),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("code-insiders --wait"),
            "EditorFailed display should include the full editor string with arguments"
        );
    }
}
