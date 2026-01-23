//! Agent Console Dashboard library
//!
//! This crate provides the core functionality for the Agent Console daemon,
//! including daemon process management and configuration.

use std::path::PathBuf;

/// Daemon module providing process lifecycle management and daemonization.
pub mod daemon;

/// Configuration for the daemon process.
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Path to the Unix socket for IPC communication.
    pub socket_path: PathBuf,
    /// Whether to run as a background daemon (detached from terminal).
    pub daemonize: bool,
}

impl DaemonConfig {
    /// Creates a new DaemonConfig with the specified socket path and daemonize flag.
    pub fn new(socket_path: PathBuf, daemonize: bool) -> Self {
        Self {
            socket_path,
            daemonize,
        }
    }
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: PathBuf::from("/tmp/agent-console.sock"),
            daemonize: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_config_default() {
        let config = DaemonConfig::default();
        assert_eq!(config.socket_path, PathBuf::from("/tmp/agent-console.sock"));
        assert!(!config.daemonize);
    }

    #[test]
    fn test_daemon_config_new() {
        let config = DaemonConfig::new(PathBuf::from("/custom/path.sock"), true);
        assert_eq!(config.socket_path, PathBuf::from("/custom/path.sock"));
        assert!(config.daemonize);
    }
}
