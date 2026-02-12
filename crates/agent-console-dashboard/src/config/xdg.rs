//! Platform-aware path resolution for agent-console-dashboard.
//!
//! On **Linux**, follows the XDG Base Directory Specification:
//! - Config: `$XDG_CONFIG_HOME/agent-console-dashboard` or `~/.config/agent-console-dashboard`
//! - Runtime/socket: `$XDG_RUNTIME_DIR` or `/tmp`
//!
//! On **macOS**, uses Apple conventions with XDG env var overrides:
//! - Config: `$XDG_CONFIG_HOME/agent-console-dashboard` or `~/Library/Application Support/agent-console-dashboard`
//! - Runtime/socket: `$XDG_RUNTIME_DIR` or `$TMPDIR` or `/tmp`

use std::fs;
use std::path::{Path, PathBuf};

const APP_NAME: &str = "agent-console-dashboard";

/// Returns the configuration directory for agent-console-dashboard.
///
/// Resolution order:
/// 1. `$XDG_CONFIG_HOME/agent-console-dashboard` (if env var set, any platform)
/// 2. Platform default:
///    - Linux: `~/.config/agent-console-dashboard`
///    - macOS: `~/Library/Application Support/agent-console-dashboard`
pub fn config_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join(APP_NAME);
    }
    platform_config_dir().join(APP_NAME)
}

/// Platform-native config base directory (without XDG override).
fn platform_config_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        // ~/Library/Application Support
        dirs::config_dir().expect("could not determine config directory")
    }
    #[cfg(not(target_os = "macos"))]
    {
        // ~/.config (XDG default on Linux)
        dirs::home_dir()
            .expect("could not determine home directory")
            .join(".config")
    }
}

/// Returns the path to the main configuration file.
///
/// Resolves to `config_dir()/config.toml`.
pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

/// Returns the runtime directory for transient files (sockets, pid files).
///
/// Resolution order:
/// 1. `$XDG_RUNTIME_DIR` (if set, any platform)
/// 2. Platform default:
///    - Linux: `/tmp` (XDG_RUNTIME_DIR is usually set by systemd)
///    - macOS: `$TMPDIR` or `/tmp`
pub fn runtime_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(xdg);
    }
    platform_runtime_dir()
}

/// Platform-native runtime directory (without XDG override).
fn platform_runtime_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        // macOS sets TMPDIR to a per-user secure directory like
        // /var/folders/xx/.../T/ — better than /tmp for sockets.
        std::env::var("TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        PathBuf::from("/tmp")
    }
}

/// Returns the path to the Unix domain socket.
///
/// Resolves to `runtime_dir()/agent-console-dashboard.sock`.
pub fn socket_path() -> PathBuf {
    runtime_dir().join(format!("{APP_NAME}.sock"))
}

/// Expands a leading `~` in a path string to the user's home directory.
///
/// If the path does not start with `~`, it is returned as-is.
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = dirs::home_dir().expect("could not determine home directory");
        home.join(rest)
    } else if path == "~" {
        dirs::home_dir().expect("could not determine home directory")
    } else {
        PathBuf::from(path)
    }
}

/// Creates a directory and all parent directories with mode 0700.
///
/// Equivalent to `mkdir -p` with restricted permissions.
pub fn ensure_dir(path: &Path) -> std::io::Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// Creates the configuration directory if it does not exist, returning its path.
pub fn ensure_config_dir() -> std::io::Result<PathBuf> {
    let dir = config_dir();
    ensure_dir(&dir)?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    /// Helper: run a closure with env vars temporarily set, then restore.
    fn with_env<F: FnOnce()>(vars: &[(&str, Option<&str>)], f: F) {
        let originals: Vec<_> = vars
            .iter()
            .map(|(k, _)| (*k, std::env::var(k).ok()))
            .collect();

        for (k, v) in vars {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }

        f();

        for (k, original) in &originals {
            match original {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
    }

    #[test]
    #[serial]
    fn test_config_path_with_xdg_override() {
        with_env(&[("XDG_CONFIG_HOME", Some("/custom/config"))], || {
            let path = config_path();
            assert_eq!(
                path,
                PathBuf::from("/custom/config/agent-console-dashboard/config.toml")
            );
        });
    }

    #[test]
    #[serial]
    fn test_config_path_without_xdg_uses_platform_default() {
        with_env(&[("XDG_CONFIG_HOME", None)], || {
            let path = config_path();
            let expected = platform_config_dir().join("agent-console-dashboard/config.toml");
            assert_eq!(path, expected);
        });
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[serial]
    fn test_macos_config_default_is_library() {
        with_env(&[("XDG_CONFIG_HOME", None)], || {
            let dir = config_dir();
            let home = dirs::home_dir().expect("could not determine home directory");
            assert_eq!(
                dir,
                home.join("Library/Application Support/agent-console-dashboard")
            );
        });
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    #[serial]
    fn test_linux_config_default_is_dot_config() {
        with_env(&[("XDG_CONFIG_HOME", None)], || {
            let dir = config_dir();
            let home = dirs::home_dir().expect("could not determine home directory");
            assert_eq!(dir, home.join(".config/agent-console-dashboard"));
        });
    }

    #[test]
    #[serial]
    fn test_runtime_dir_with_xdg_override() {
        with_env(&[("XDG_RUNTIME_DIR", Some("/run/user/1000"))], || {
            let dir = runtime_dir();
            assert_eq!(dir, PathBuf::from("/run/user/1000"));
        });
    }

    #[test]
    #[serial]
    fn test_runtime_dir_without_xdg_uses_platform_default() {
        with_env(&[("XDG_RUNTIME_DIR", None)], || {
            let dir = runtime_dir();
            let expected = platform_runtime_dir();
            assert_eq!(dir, expected);
        });
    }

    #[test]
    #[serial]
    fn test_socket_path_with_xdg_override() {
        with_env(&[("XDG_RUNTIME_DIR", Some("/run/user/1000"))], || {
            let path = socket_path();
            assert_eq!(
                path,
                PathBuf::from("/run/user/1000/agent-console-dashboard.sock")
            );
        });
    }

    #[test]
    #[serial]
    fn test_config_dir_with_xdg_override() {
        with_env(&[("XDG_CONFIG_HOME", Some("/custom/config"))], || {
            let dir = config_dir();
            assert_eq!(dir, PathBuf::from("/custom/config/agent-console-dashboard"));
        });
    }

    #[test]
    fn test_expand_tilde_with_home_prefix() {
        let home = dirs::home_dir().expect("could not determine home directory");
        let result = expand_tilde("~/foo");
        assert_eq!(result, home.join("foo"));
    }

    #[test]
    fn test_expand_tilde_absolute_path_unchanged() {
        let result = expand_tilde("/absolute/path");
        assert_eq!(result, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_expand_tilde_bare_tilde() {
        let home = dirs::home_dir().expect("could not determine home directory");
        let result = expand_tilde("~");
        assert_eq!(result, home);
    }

    #[test]
    fn test_expand_tilde_relative_path() {
        let result = expand_tilde("relative/path");
        assert_eq!(result, PathBuf::from("relative/path"));
    }

    #[test]
    fn test_ensure_dir_creates_directory() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        let nested = tmp.path().join("a/b/c");
        ensure_dir(&nested).expect("ensure_dir failed");
        assert!(nested.is_dir());
    }

    #[test]
    fn test_ensure_dir_sets_permissions() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let tmp = tempfile::tempdir().expect("failed to create temp dir");
            let dir = tmp.path().join("secure");
            ensure_dir(&dir).expect("ensure_dir failed");
            let mode = fs::metadata(&dir)
                .expect("failed to read metadata")
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o700);
        }
    }

    #[test]
    #[serial]
    fn test_ensure_config_dir_creates_at_xdg_path() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        with_env(
            &[(
                "XDG_CONFIG_HOME",
                Some(tmp.path().to_str().expect("non-utf8 tmpdir")),
            )],
            || {
                let result = ensure_config_dir().expect("ensure_config_dir failed");
                assert_eq!(result, tmp.path().join("agent-console-dashboard"));
                assert!(result.is_dir());
            },
        );
    }
}
