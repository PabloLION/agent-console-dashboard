//! TOML configuration schema types for the Agent Console Dashboard.
//!
//! All structs derive `Deserialize` and `Serialize` with sensible defaults via
//! `#[serde(default)]`. Fields are annotated with hot-reload behavior in doc comments.
//!
//! Duration fields use human-readable strings (e.g. `"60m"`, `"3m"`, `"250ms"`)
//! parsed by the `humantime` crate at the call site.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Top-level Config
// ---------------------------------------------------------------------------

/// Root configuration encompassing all sections.
///
/// Corresponds to the full TOML file structure:
/// ```toml
/// [tui]
/// [agents]
/// [integrations]
/// [daemon]
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct Config {
    /// TUI appearance and behavior settings.
    pub tui: TuiConfig,
    /// Agent-specific configuration.
    pub agents: AgentsConfig,
    /// Third-party integration settings.
    pub integrations: IntegrationsConfig,
    /// Daemon process settings.
    pub daemon: TomlDaemonConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tui: TuiConfig::default(),
            agents: AgentsConfig::default(),
            integrations: IntegrationsConfig::default(),
            daemon: TomlDaemonConfig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// TUI
// ---------------------------------------------------------------------------

/// TUI layout and widget configuration.
///
/// Hot-reloadable: Yes (all fields).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct TuiConfig {
    /// Active layout preset. Hot-reloadable: Yes.
    pub layout: LayoutPreset,
    /// Ordered list of widget identifiers to display.
    /// Hot-reloadable: Yes.
    pub widgets: Vec<String>,
    /// Render tick rate as a human-readable duration (e.g. `"250ms"`).
    /// Hot-reloadable: No (restart required).
    pub tick_rate: String,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            layout: LayoutPreset::Default,
            widgets: vec![
                "session-status:two-line".to_string(),
                "api-usage".to_string(),
            ],
            tick_rate: "250ms".to_string(),
        }
    }
}

/// Layout preset variants.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum LayoutPreset {
    /// Full-featured layout with all panels.
    Default,
    /// Reduced-height layout for smaller terminals.
    Compact,
}

// ---------------------------------------------------------------------------
// Agents
// ---------------------------------------------------------------------------

/// Agent-level configuration container.
///
/// Each supported agent has its own sub-section (e.g. `[agents.claude-code]`).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct AgentsConfig {
    /// Claude Code agent settings.
    #[serde(rename = "claude-code")]
    pub claude_code: ClaudeCodeConfig,
}

impl Default for AgentsConfig {
    fn default() -> Self {
        Self {
            claude_code: ClaudeCodeConfig::default(),
        }
    }
}

/// Configuration for the Claude Code agent integration.
///
/// Hot-reloadable: No (restart required for both fields).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct ClaudeCodeConfig {
    /// Whether Claude Code integration is active.
    pub enabled: bool,
    /// Path to the Claude Code hooks directory.
    pub hooks_path: String,
}

impl Default for ClaudeCodeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            hooks_path: "~/.claude/hooks".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Integrations
// ---------------------------------------------------------------------------

/// Third-party integration configuration.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct IntegrationsConfig {
    /// Zellij terminal multiplexer integration.
    pub zellij: ZellijConfig,
}

impl Default for IntegrationsConfig {
    fn default() -> Self {
        Self {
            zellij: ZellijConfig::default(),
        }
    }
}

/// Zellij integration configuration.
///
/// Hot-reloadable: No (restart required).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct ZellijConfig {
    /// Whether Zellij integration is active.
    pub enabled: bool,
}

impl Default for ZellijConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

// ---------------------------------------------------------------------------
// Daemon
// ---------------------------------------------------------------------------

/// Daemon process configuration from the TOML `[daemon]` section.
///
/// Named `TomlDaemonConfig` to avoid collision with the runtime
/// `crate::DaemonConfig` (socket path / daemonize flag).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct TomlDaemonConfig {
    /// Auto-stop after idle period (default: `"60m"` per D5 amendment).
    /// Hot-reloadable: Yes.
    pub idle_timeout: String,
    /// API usage fetch interval (default: `"3m"` per D4).
    /// Hot-reloadable: Yes.
    pub usage_fetch_interval: String,
    /// Logging verbosity. Hot-reloadable: Yes.
    pub log_level: LogLevel,
    /// Path to log file. Empty string means stderr.
    /// Hot-reloadable: No (restart required).
    pub log_file: String,
}

impl Default for TomlDaemonConfig {
    fn default() -> Self {
        Self {
            idle_timeout: "60m".to_string(),
            usage_fetch_interval: "3m".to_string(),
            log_level: LogLevel::Info,
            log_file: String::new(),
        }
    }
}

/// Log verbosity levels (kebab-case in TOML).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum LogLevel {
    /// Only errors.
    Error,
    /// Errors and warnings.
    Warn,
    /// Informational messages (default).
    Info,
    /// Debug-level detail.
    Debug,
    /// Full trace output.
    Trace,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_config_all_fields() {
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
        let config: Config = toml::from_str(toml_str).expect("valid TOML should parse");
        assert_eq!(config.tui.layout, LayoutPreset::Compact);
        assert_eq!(config.tui.widgets, vec!["session-status:one-line"]);
        assert_eq!(config.tui.tick_rate, "100ms");
        assert!(!config.agents.claude_code.enabled);
        assert_eq!(config.agents.claude_code.hooks_path, "/custom/hooks");
        assert!(!config.integrations.zellij.enabled);
        assert_eq!(config.daemon.idle_timeout, "30m");
        assert_eq!(config.daemon.usage_fetch_interval, "5m");
        assert_eq!(config.daemon.log_level, LogLevel::Debug);
        assert_eq!(config.daemon.log_file, "/var/log/acd.log");
    }

    #[test]
    fn parse_empty_string_uses_all_defaults() {
        let config: Config = toml::from_str("").expect("empty string should parse");
        let defaults = Config::default();
        assert_eq!(config, defaults);
    }

    #[test]
    fn parse_unknown_fields_are_ignored() {
        let toml_str = r#"
unknown_key = "hello"

[tui]
future_field = 42
"#;
        let config: Config = toml::from_str(toml_str).expect("unknown fields should be ignored");
        assert_eq!(config.tui.layout, LayoutPreset::Default);
    }

    #[test]
    fn default_idle_timeout_is_60m() {
        let config = Config::default();
        assert_eq!(config.daemon.idle_timeout, "60m");
    }

    #[test]
    fn default_usage_fetch_interval_is_3m() {
        let config = Config::default();
        assert_eq!(config.daemon.usage_fetch_interval, "3m");
    }

    #[test]
    fn layout_preset_parsing() {
        let toml_default = r#"layout = "default""#;
        let toml_compact = r#"layout = "compact""#;

        let tui_default: TuiConfig =
            toml::from_str(toml_default).expect("default preset should parse");
        let tui_compact: TuiConfig =
            toml::from_str(toml_compact).expect("compact preset should parse");

        assert_eq!(tui_default.layout, LayoutPreset::Default);
        assert_eq!(tui_compact.layout, LayoutPreset::Compact);
    }

    #[test]
    fn invalid_layout_preset_returns_error() {
        let toml_str = r#"layout = "nonexistent""#;
        let result: Result<TuiConfig, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn log_level_all_variants() {
        for (input, expected) in [
            ("error", LogLevel::Error),
            ("warn", LogLevel::Warn),
            ("info", LogLevel::Info),
            ("debug", LogLevel::Debug),
            ("trace", LogLevel::Trace),
        ] {
            let toml_str = format!("log_level = \"{}\"", input);
            let daemon: TomlDaemonConfig =
                toml::from_str(&toml_str).expect("log level should parse");
            assert_eq!(daemon.log_level, expected);
        }
    }

    #[test]
    fn invalid_log_level_returns_error() {
        let toml_str = r#"log_level = "verbose""#;
        let result: Result<TomlDaemonConfig, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn roundtrip_serialize_deserialize() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).expect("serialization should succeed");
        let parsed: Config = toml::from_str(&toml_str).expect("roundtrip should parse");
        assert_eq!(config, parsed);
    }

    #[test]
    fn default_widgets_list() {
        let config = Config::default();
        assert_eq!(
            config.tui.widgets,
            vec!["session-status:two-line", "api-usage"]
        );
    }

    #[test]
    fn default_tick_rate() {
        let config = Config::default();
        assert_eq!(config.tui.tick_rate, "250ms");
    }

    #[test]
    fn default_claude_code_enabled() {
        let config = Config::default();
        assert!(config.agents.claude_code.enabled);
        assert_eq!(config.agents.claude_code.hooks_path, "~/.claude/hooks");
    }

    #[test]
    fn default_zellij_enabled() {
        let config = Config::default();
        assert!(config.integrations.zellij.enabled);
    }

    #[test]
    fn default_log_level_is_info() {
        let config = Config::default();
        assert_eq!(config.daemon.log_level, LogLevel::Info);
    }

    #[test]
    fn default_log_file_is_empty() {
        let config = Config::default();
        assert_eq!(config.daemon.log_file, "");
    }

    #[test]
    fn partial_config_fills_defaults() {
        let toml_str = r#"
[daemon]
log_level = "debug"
"#;
        let config: Config = toml::from_str(toml_str).expect("partial config should parse");
        assert_eq!(config.daemon.log_level, LogLevel::Debug);
        // All other fields should be defaults
        assert_eq!(config.daemon.idle_timeout, "60m");
        assert_eq!(config.daemon.usage_fetch_interval, "3m");
        assert_eq!(config.tui.layout, LayoutPreset::Default);
    }
}
