//! Default configuration template and file creation utilities.
//!
//! Provides a well-commented TOML template that matches `Config::default()`
//! and functions to write it to the XDG config path.

use std::fs;
use std::path::PathBuf;

use crate::config::error::ConfigError;
use crate::config::xdg;

// ---------------------------------------------------------------------------
// Timestamp utilities
// ---------------------------------------------------------------------------

/// Generates a tinydate timestamp in Zulu (UTC) format: YYYYMMDDTHHmmssZ
///
/// Example: `20260213T143052Z` (16 characters)
pub fn generate_tinydate() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

// ---------------------------------------------------------------------------
// Default TOML template
// ---------------------------------------------------------------------------

/// A well-commented TOML template with all default values.
///
/// Every value here must match `Config::default()` from `schema.rs`.
/// Sections: `[tui]`, `[agents.claude-code]`, `[integrations.zellij]`, `[daemon]`.
pub const DEFAULT_CONFIG_TEMPLATE: &str = r#"# Agent Console Dashboard Configuration
#
# This file was auto-generated with default values.
# All values shown below are the built-in defaults.
# Uncomment and modify options to customize your dashboard.
#
# Location: $XDG_CONFIG_HOME/agent-console-dashboard/config.toml
# Reference: https://github.com/PabloLION/agent-console-dashboard

# ==============================================================================
# TUI Configuration
# ==============================================================================

[tui]

# Layout preset for the dashboard.
# Options: "default", "compact"
#   default - Full-featured layout with all panels
#   compact - Reduced-height layout for smaller terminals
layout = "default"

# Ordered list of widgets to display in the dashboard.
# Each entry is a widget identifier, optionally with a variant suffix.
# Available widgets: "session-status:two-line", "session-status:one-line", "api-usage"
widgets = ["session-status:two-line", "api-usage"]

# Render tick rate as a human-readable duration.
# Controls how often the TUI redraws. Lower values = smoother but more CPU.
# Examples: "250ms", "500ms", "1s"
# Note: Changing this requires a restart (not hot-reloadable).
tick_rate = "250ms"

# Shell command to run on double-click of active session (activate action).
# Fires when double-clicking a non-closed session.
# Supports placeholders: {session_id}, {working_dir}, {status}
# Executed via `sh -c` (fire-and-forget, no callback).
# Empty string means double-click has no effect.
# Hot-reloadable: Yes
#
# Examples:
#   Zellij — focus the tab matching the folder name:
#     "zellij action go-to-tab-name $(basename {working_dir})"
#   VS Code — open the folder:
#     "code {working_dir}"
#   tmux — switch to window matching the folder name:
#     "tmux select-window -t $(basename {working_dir})"
#   Terminal — open a new terminal window at the folder:
#     "open -a Terminal {working_dir}"
activate_hook = ""

# Shell command to run on double-click of closed session (reopen action).
# Fires when double-clicking a closed session.
# Supports placeholders: {session_id}, {working_dir}, {status}
# Executed via `sh -c` (fire-and-forget, no callback).
# Empty string means double-click has no effect.
# Hot-reloadable: Yes
#
# Examples:
#   Zellij — focus the tab matching the folder name:
#     "zellij action go-to-tab-name $(basename {working_dir})"
#   Zellij — open a new tab at the folder:
#     "zellij action new-tab --name $(basename {working_dir}) --cwd {working_dir}"
#   tmux — create a new window at the folder:
#     "tmux new-window -n $(basename {working_dir}) -c {working_dir}"
reopen_hook = ""

# ==============================================================================
# Agent Configuration
# ==============================================================================

[agents.claude-code]

# Enable Claude Code integration.
# Set to false to disable Claude Code session tracking entirely.
enabled = true

# Path to the Claude Code hooks directory.
# Tilde (~) is expanded to the user's home directory.
hooks_path = "~/.claude/hooks"

# ==============================================================================
# Integration Configuration
# ==============================================================================

[integrations.zellij]

# Enable Zellij terminal multiplexer integration.
# When enabled, supports session resurrection via Zellij panes.
enabled = true

# ==============================================================================
# Daemon Configuration
# ==============================================================================

[daemon]

# Auto-stop the daemon after this idle duration with no active sessions.
# Set to a longer value if you want the daemon to persist.
# Examples: "60m", "2h", "30m"
# Hot-reloadable: Yes
idle_timeout = "60m"

# Interval between API usage data fetches.
# Lower values give fresher data but increase API calls.
# Examples: "3m", "5m", "1m"
# Hot-reloadable: Yes
usage_fetch_interval = "3m"

# Logging verbosity level.
# Options: "error", "warn", "info", "debug", "trace"
#   error - Only errors
#   warn  - Errors and warnings
#   info  - General operational information (recommended)
#   debug - Detailed debugging information
#   trace - Very verbose, includes all internal operations
# Hot-reloadable: Yes
log_level = "info"

# Path to log file. Empty string uses the default XDG state path.
# Default: ~/.local/state/agent-console-dashboard/daemon.log
# Tilde (~) is expanded to your home directory.
# Examples: "/var/log/agent-console-dashboard.log", "~/logs/acd-daemon.log"
# Hot-reloadable: No (restart required)
log_file = ""
"#;

// ---------------------------------------------------------------------------
// File creation functions
// ---------------------------------------------------------------------------

/// Creates the default config file if it does not already exist.
///
/// Returns `Ok(true)` if the file was created, `Ok(false)` if it already exists.
/// Uses `xdg::config_path()` for the target location and creates parent
/// directories via `xdg::ensure_config_dir()`.
pub fn create_default_config_if_missing() -> Result<bool, ConfigError> {
    let path = xdg::config_path();

    if path.exists() {
        return Ok(false);
    }

    write_default_config(&path)?;
    tracing::info!("Created default configuration at {}", path.display());
    Ok(true)
}

/// Creates (or force-overwrites) the default config file.
///
/// - If the file exists and `force` is `false`, returns `ConfigError::AlreadyExists`.
/// - If the file exists and `force` is `true`, backs it up to `<name>.bak.<tinydate>` first.
/// - Returns the path where the config was written.
///
/// When `force` is true but the config doesn't exist, behaves like normal creation
/// (no backup, just creates the file).
pub fn create_default_config(force: bool) -> Result<PathBuf, ConfigError> {
    let path = xdg::config_path();

    if path.exists() {
        if !force {
            return Err(ConfigError::AlreadyExists { path: path.clone() });
        }
        // Back up existing file with tinydate format: config.toml.bak.YYYYMMDDTHHmmssZ
        let tinydate = generate_tinydate();
        let backup_path = PathBuf::from(format!("{}.bak.{}", path.display(), tinydate));
        fs::rename(&path, &backup_path).map_err(|e| ConfigError::WriteError {
            path: backup_path.clone(),
            source: e,
        })?;
        println!("Old config backed up to: {}", backup_path.display());
        tracing::info!("Backed up existing config to {}", backup_path.display());
    }

    write_default_config(&path)?;
    println!("Created new config: {}", path.display());
    Ok(path)
}

/// Writes the default template to `path`, creating parent dirs and setting 0600 permissions.
fn write_default_config(path: &PathBuf) -> Result<(), ConfigError> {
    xdg::ensure_config_dir().map_err(|e| ConfigError::WriteError {
        path: path.clone(),
        source: e,
    })?;

    fs::write(path, DEFAULT_CONFIG_TEMPLATE).map_err(|e| ConfigError::WriteError {
        path: path.clone(),
        source: e,
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| {
            ConfigError::WriteError {
                path: path.clone(),
                source: e,
            }
        })?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::Config;
    use serial_test::serial;

    /// Run closure with `XDG_CONFIG_HOME` temporarily pointed at `dir`.
    fn with_xdg_config<F: FnOnce()>(dir: &str, f: F) {
        let original = std::env::var("XDG_CONFIG_HOME").ok();
        unsafe { std::env::set_var("XDG_CONFIG_HOME", dir) };
        f();
        match original {
            Some(v) => unsafe { std::env::set_var("XDG_CONFIG_HOME", v) },
            None => unsafe { std::env::remove_var("XDG_CONFIG_HOME") },
        }
    }

    // -- Template validity --------------------------------------------------

    #[test]
    fn template_parses_to_valid_config() {
        let config: Config =
            toml::from_str(DEFAULT_CONFIG_TEMPLATE).expect("template should parse");
        // Sanity: at least one field is populated
        assert_eq!(config.tui.tick_rate, "250ms");
    }

    #[test]
    fn template_values_match_config_default() {
        let from_template: Config =
            toml::from_str(DEFAULT_CONFIG_TEMPLATE).expect("template should parse");
        let defaults = Config::default();
        assert_eq!(from_template, defaults);
    }

    #[test]
    fn template_contains_all_section_headers() {
        assert!(
            DEFAULT_CONFIG_TEMPLATE.contains("[tui]"),
            "missing [tui] section"
        );
        assert!(
            DEFAULT_CONFIG_TEMPLATE.contains("[agents.claude-code]"),
            "missing [agents.claude-code] section"
        );
        assert!(
            DEFAULT_CONFIG_TEMPLATE.contains("[integrations.zellij]"),
            "missing [integrations.zellij] section"
        );
        assert!(
            DEFAULT_CONFIG_TEMPLATE.contains("[daemon]"),
            "missing [daemon] section"
        );
    }

    #[test]
    fn template_is_heavily_commented() {
        let comment_lines = DEFAULT_CONFIG_TEMPLATE
            .lines()
            .filter(|l| l.trim_start().starts_with('#'))
            .count();
        // Should have significantly more comment lines than value lines
        assert!(
            comment_lines > 20,
            "expected >20 comment lines, got {comment_lines}"
        );
    }

    // -- Timestamp tests ----------------------------------------------------

    #[test]
    fn tinydate_format_is_correct() {
        let tinydate = generate_tinydate();
        assert_eq!(tinydate.len(), 16, "tinydate should be 16 characters");
        assert!(tinydate.ends_with('Z'), "tinydate should end with Z");
        assert!(
            tinydate.contains('T'),
            "tinydate should contain T separator"
        );

        // Format: YYYYMMDDTHHmmssZ
        // Verify it parses as expected structure
        let parts: Vec<&str> = tinydate.split('T').collect();
        assert_eq!(parts.len(), 2, "should split into date and time parts");
        assert_eq!(parts[0].len(), 8, "date part should be 8 chars (YYYYMMDD)");
        assert_eq!(parts[1].len(), 7, "time part should be 7 chars (HHmmssZ)");

        // Verify all chars except T and Z are digits
        let digits_only: String = tinydate
            .chars()
            .filter(|c| *c != 'T' && *c != 'Z')
            .collect();
        assert!(
            digits_only.chars().all(|c| c.is_ascii_digit()),
            "all chars except T and Z should be digits"
        );
    }

    // -- create_default_config_if_missing -----------------------------------

    #[test]
    #[serial]
    fn if_missing_creates_file() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        let expected_path = tmp.path().join("agent-console-dashboard/config.toml");
        with_xdg_config(tmp.path().to_str().expect("non-utf8 tmpdir"), || {
            let created = create_default_config_if_missing().expect("should succeed");
            assert!(created, "should report file was created");
            assert!(expected_path.exists(), "config file should exist on disk");
            let content = fs::read_to_string(&expected_path).expect("should read");
            assert_eq!(content, DEFAULT_CONFIG_TEMPLATE);
        });
    }

    #[test]
    #[serial]
    fn if_missing_returns_false_when_exists() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        // Pre-create the file so we don't depend on two sequential calls
        // both seeing the same XDG_CONFIG_HOME
        let config_dir = tmp.path().join("agent-console-dashboard");
        fs::create_dir_all(&config_dir).expect("create config dir");
        let config_file = config_dir.join("config.toml");
        fs::write(&config_file, DEFAULT_CONFIG_TEMPLATE).expect("write initial config");

        with_xdg_config(tmp.path().to_str().expect("non-utf8 tmpdir"), || {
            let created = create_default_config_if_missing().expect("should succeed");
            assert!(!created, "should report file was NOT created");
        });
    }

    // -- create_default_config ----------------------------------------------

    #[test]
    #[serial]
    fn create_without_force_returns_already_exists() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        with_xdg_config(tmp.path().to_str().expect("non-utf8 tmpdir"), || {
            // Create initial file
            create_default_config(false).expect("first call should succeed");
            // Try again without force
            let err = create_default_config(false).expect_err("should fail with AlreadyExists");
            match err {
                ConfigError::AlreadyExists { .. } => {}
                other => panic!("expected AlreadyExists, got: {other:?}"),
            }
        });
    }

    #[test]
    #[serial]
    fn create_with_force_creates_backup() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        with_xdg_config(tmp.path().to_str().expect("non-utf8 tmpdir"), || {
            // Create initial file with custom content
            let path = create_default_config(false).expect("first call should succeed");
            fs::write(&path, "# custom content\n").expect("overwrite for test");

            // Force overwrite
            let new_path = create_default_config(true).expect("force should succeed");
            assert_eq!(new_path, path);

            // Backup should exist with format: config.toml.bak.YYYYMMDDTHHmmssZ
            let parent = path.parent().expect("config file should have parent dir");
            let backups: Vec<_> = fs::read_dir(parent)
                .expect("read config dir")
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .map(|n| n.starts_with("config.toml.bak."))
                        .unwrap_or(false)
                })
                .collect();
            assert_eq!(backups.len(), 1, "should have exactly one backup file");
            let backup_path = backups[0].path();

            // Verify backup filename format: config.toml.bak.YYYYMMDDTHHmmssZ (16 char timestamp)
            let backup_name = backup_path.file_name().expect("backup has filename");
            let backup_str = backup_name.to_str().expect("filename is utf8");
            assert!(
                backup_str.starts_with("config.toml.bak."),
                "backup name should start with config.toml.bak."
            );
            let timestamp = backup_str
                .strip_prefix("config.toml.bak.")
                .expect("strip prefix");
            assert_eq!(
                timestamp.len(),
                16,
                "tinydate timestamp should be 16 chars (YYYYMMDDTHHmmssZ)"
            );
            assert!(
                timestamp.ends_with('Z'),
                "tinydate should end with Z (Zulu time)"
            );

            let backup_content = fs::read_to_string(&backup_path).expect("read backup");
            assert_eq!(backup_content, "# custom content\n");

            // New file should be template
            let content = fs::read_to_string(&path).expect("read new");
            assert_eq!(content, DEFAULT_CONFIG_TEMPLATE);
        });
    }

    #[test]
    #[serial]
    fn create_with_force_when_no_existing_config() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        with_xdg_config(tmp.path().to_str().expect("non-utf8 tmpdir"), || {
            // Call with force=true when no config exists
            let path = create_default_config(true).expect("should succeed");

            // Should create config normally (no backup)
            assert!(path.exists(), "config file should exist");
            let content = fs::read_to_string(&path).expect("should read");
            assert_eq!(content, DEFAULT_CONFIG_TEMPLATE);

            // No backup files should exist
            let parent = path.parent().expect("config file should have parent dir");
            let backups: Vec<_> = fs::read_dir(parent)
                .expect("read config dir")
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .map(|n| n.starts_with("config.toml.bak."))
                        .unwrap_or(false)
                })
                .collect();
            assert_eq!(backups.len(), 0, "should have no backup files");
        });
    }

    #[test]
    #[serial]
    fn create_returns_correct_path() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        let expected = tmp.path().join("agent-console-dashboard/config.toml");
        with_xdg_config(tmp.path().to_str().expect("non-utf8 tmpdir"), || {
            let path = create_default_config(false).expect("should succeed");
            assert_eq!(path, expected);
        });
    }

    // -- Permissions --------------------------------------------------------

    #[cfg(unix)]
    #[test]
    #[serial]
    fn file_permissions_are_0600() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        let expected_path = tmp.path().join("agent-console-dashboard/config.toml");
        with_xdg_config(tmp.path().to_str().expect("non-utf8 tmpdir"), || {
            create_default_config_if_missing().expect("should succeed");
            let mode = fs::metadata(&expected_path)
                .expect("metadata")
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600, "file should be owner-only read/write");
        });
    }

    // -- generate_tinydate --------------------------------------------------

    #[test]
    fn generate_tinydate_is_public() {
        // Verify that generate_tinydate is accessible (public)
        let _tinydate = generate_tinydate();
        // If this compiles, the function is public
    }
}
