//! Settings file I/O and manipulation
//!
//! This module handles reading and writing Claude's settings.json with atomic
//! safety guarantees. It preserves all non-hook fields while modifying the
//! hooks array.

use crate::error::{Result, SettingsError};
use crate::types::{HookEvent, HookHandler};
use chrono::Local;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

/// Returns the path to Claude's user settings.json
///
/// Location: `~/.claude/settings.json`
pub fn settings_path() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME environment variable not set");
    PathBuf::from(home).join(".claude").join("settings.json")
}

/// Read settings.json and parse as Value (preserves all fields)
///
/// Parses the entire settings.json file as a `serde_json::Value` to preserve
/// all top-level keys per D13 (cleanupPeriodDays, env, permissions, etc.).
///
/// # Errors
///
/// Returns `SettingsError::Io` if file cannot be read.
/// Returns `SettingsError::Parse` if JSON is malformed.
pub fn read_settings() -> Result<Value> {
    let path = settings_path();
    let content = fs::read_to_string(&path).map_err(SettingsError::Io)?;

    serde_json::from_str(&content).map_err(|e| SettingsError::Parse(e.to_string()).into())
}

/// Write settings.json atomically with temp-file-then-rename
///
/// Implements atomic write pattern (D01):
/// 1. Write to temp file with timestamp suffix
/// 2. Fsync to disk
/// 3. Rename temp to original (atomic operation)
///
/// On failure before rename, temp file is preserved as "safety copy".
///
/// # Errors
///
/// Returns `SettingsError::Parse` if value cannot be serialized.
/// Returns `SettingsError::Io` if file cannot be written or synced.
/// Returns `SettingsError::WriteAtomic` if rename fails.
pub fn write_settings_atomic(value: Value) -> Result<()> {
    let path = settings_path();
    let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let temp_path = path.with_file_name(format!("settings.json.tmp.{}", timestamp));

    // Write to temp file
    let json = serde_json::to_string_pretty(&value)
        .map_err(|e| SettingsError::Parse(e.to_string()))?;

    fs::write(&temp_path, json).map_err(SettingsError::Io)?;

    // Fsync (ensure data is on disk)
    let file = fs::File::open(&temp_path).map_err(SettingsError::Io)?;
    file.sync_all().map_err(SettingsError::Io)?;

    // Atomic rename
    fs::rename(&temp_path, &path).map_err(|_| SettingsError::WriteAtomic {
        path: path.clone(),
        temp_path: temp_path.clone(),
    })?;

    Ok(())
}

/// Add hook to hooks array (pure function, no I/O)
///
/// Inserts a new hook into the hooks array. This is a pure function that
/// returns a modified copy of the value.
///
/// # Panics
///
/// Panics if settings.json is missing 'hooks' array or if serialization fails.
/// These conditions indicate a malformed settings file.
pub fn add_hook(mut value: Value, event: HookEvent, handler: HookHandler) -> Value {
    let hooks_array = value
        .get_mut("hooks")
        .and_then(|h| h.as_array_mut())
        .expect("settings.json missing 'hooks' array");

    // Serialize handler to JSON
    let mut hook_obj = serde_json::to_value(&handler)
        .expect("handler serialization failed")
        .as_object()
        .expect("handler should serialize to object")
        .clone();

    // Add event field
    hook_obj.insert(
        "event".to_string(),
        serde_json::to_value(event).expect("event serialization failed"),
    );

    hooks_array.push(Value::Object(hook_obj));
    value
}

/// Remove hook from hooks array by exact match (pure function, no I/O)
///
/// Removes hooks that match both event and command. This is a pure function
/// that returns a modified copy of the value.
///
/// # Panics
///
/// Panics if settings.json is missing 'hooks' array.
pub fn remove_hook(mut value: Value, event: HookEvent, command: &str) -> Value {
    let hooks_array = value
        .get_mut("hooks")
        .and_then(|h| h.as_array_mut())
        .expect("settings.json missing 'hooks' array");

    hooks_array.retain(|hook| {
        let hook_event = hook
            .get("event")
            .and_then(|e| serde_json::from_value(e.clone()).ok());
        let hook_command = hook.get("command").and_then(|c| c.as_str());

        // Keep if event or command doesn't match
        hook_event != Some(event) || hook_command != Some(command)
    });

    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serial_test::serial;

    #[test]
    fn test_settings_path() {
        let path = settings_path();
        assert!(path.ends_with(".claude/settings.json"));
    }

    #[test]
    fn test_add_hook_to_empty_array() {
        let settings = json!({
            "hooks": [],
            "cleanupPeriodDays": 7
        });

        let handler = HookHandler {
            r#type: "command".to_string(),
            command: "/path/to/stop.sh".to_string(),
            matcher: String::new(),
            timeout: Some(600),
            r#async: None,
        };

        let result = add_hook(settings, HookEvent::Stop, handler);
        let hooks = result.get("hooks").expect("hooks array should exist");
        let hooks_array = hooks.as_array().expect("hooks should be array");

        assert_eq!(hooks_array.len(), 1);
        assert_eq!(
            hooks_array[0].get("event").expect("event should exist"),
            "Stop"
        );
        assert_eq!(
            hooks_array[0]
                .get("command")
                .expect("command should exist"),
            "/path/to/stop.sh"
        );
        assert_eq!(
            hooks_array[0].get("type").expect("type should exist"),
            "command"
        );
    }

    #[test]
    fn test_add_hook_to_existing_array() {
        let settings = json!({
            "hooks": [
                {
                    "event": "Start",
                    "command": "/path/to/start.sh",
                    "type": "command",
                    "matcher": ""
                }
            ],
            "cleanupPeriodDays": 7
        });

        let handler = HookHandler {
            r#type: "command".to_string(),
            command: "/path/to/stop.sh".to_string(),
            matcher: String::new(),
            timeout: Some(600),
            r#async: None,
        };

        let result = add_hook(settings, HookEvent::Stop, handler);
        let hooks = result.get("hooks").expect("hooks array should exist");
        let hooks_array = hooks.as_array().expect("hooks should be array");

        assert_eq!(hooks_array.len(), 2);
        assert_eq!(
            hooks_array[1].get("event").expect("event should exist"),
            "Stop"
        );
    }

    #[test]
    fn test_remove_hook_exact_match() {
        let settings = json!({
            "hooks": [
                {
                    "event": "Stop",
                    "command": "/path/to/stop.sh",
                    "type": "command",
                    "matcher": ""
                },
                {
                    "event": "Start",
                    "command": "/path/to/start.sh",
                    "type": "command",
                    "matcher": ""
                }
            ]
        });

        let result = remove_hook(settings, HookEvent::Stop, "/path/to/stop.sh");
        let hooks = result.get("hooks").expect("hooks array should exist");
        let hooks_array = hooks.as_array().expect("hooks should be array");

        assert_eq!(hooks_array.len(), 1);
        assert_eq!(
            hooks_array[0].get("event").expect("event should exist"),
            "Start"
        );
    }

    #[test]
    fn test_remove_hook_preserves_other_hooks() {
        let settings = json!({
            "hooks": [
                {
                    "event": "Stop",
                    "command": "/path/to/stop.sh",
                    "type": "command",
                    "matcher": ""
                },
                {
                    "event": "Stop",
                    "command": "/different/path.sh",
                    "type": "command",
                    "matcher": ""
                },
                {
                    "event": "Start",
                    "command": "/path/to/start.sh",
                    "type": "command",
                    "matcher": ""
                }
            ]
        });

        let result = remove_hook(settings, HookEvent::Stop, "/path/to/stop.sh");
        let hooks = result.get("hooks").expect("hooks array should exist");
        let hooks_array = hooks.as_array().expect("hooks should be array");

        assert_eq!(hooks_array.len(), 2);
        // Should preserve the Stop hook with different command
        assert_eq!(
            hooks_array[0].get("event").expect("event should exist"),
            "Stop"
        );
        assert_eq!(
            hooks_array[0]
                .get("command")
                .expect("command should exist"),
            "/different/path.sh"
        );
        // Should preserve the Start hook
        assert_eq!(
            hooks_array[1].get("event").expect("event should exist"),
            "Start"
        );
    }

    #[test]
    fn test_roundtrip_preserves_non_hook_keys() {
        let settings = json!({
            "hooks": [],
            "cleanupPeriodDays": 7,
            "env": {"TEST": "value"},
            "permissions": {},
            "statusLine": true,
            "enabledPlugins": ["plugin1"],
            "syntaxHighlightingDisabled": false
        });

        // Serialize and deserialize
        let json_str = serde_json::to_string(&settings).expect("serialization failed");
        let parsed: Value = serde_json::from_str(&json_str).expect("deserialization failed");

        assert_eq!(
            parsed.get("cleanupPeriodDays").expect("should exist"),
            7
        );
        assert!(parsed.get("env").is_some());
        assert!(parsed.get("permissions").is_some());
        assert!(parsed.get("statusLine").is_some());
        assert!(parsed.get("enabledPlugins").is_some());
        assert!(parsed.get("syntaxHighlightingDisabled").is_some());
    }

    #[test]
    fn test_atomic_write_with_tempfile() {
        use std::io::Write;
        use tempfile::tempdir;

        // Create temp directory for test
        let dir = tempdir().expect("tempdir creation failed");
        let settings_file = dir.path().join("settings.json");

        // Write initial settings
        let initial_settings = json!({"hooks": [], "cleanupPeriodDays": 7});
        let mut file = fs::File::create(&settings_file).expect("file creation failed");
        file.write_all(
            serde_json::to_string_pretty(&initial_settings)
                .expect("serialization failed")
                .as_bytes(),
        )
        .expect("write failed");
        file.sync_all().expect("sync failed");

        // Verify initial content
        let content = fs::read_to_string(&settings_file).expect("read failed");
        let parsed: Value = serde_json::from_str(&content).expect("parse failed");
        assert_eq!(parsed.get("cleanupPeriodDays").expect("should exist"), 7);

        // This test demonstrates the pattern but doesn't test the actual
        // write_settings_atomic function since it uses hardcoded paths
        // (that's tested in integration tests)
    }

    #[test]
    fn test_add_hook_with_optional_fields() {
        let settings = json!({
            "hooks": []
        });

        let handler = HookHandler {
            r#type: "command".to_string(),
            command: "/path/to/script.sh".to_string(),
            matcher: String::new(),
            timeout: Some(300),
            r#async: Some(true),
        };

        let result = add_hook(settings, HookEvent::BeforePrompt, handler);
        let hooks = result.get("hooks").expect("hooks array should exist");
        let hooks_array = hooks.as_array().expect("hooks should be array");

        assert_eq!(hooks_array.len(), 1);
        let hook = &hooks_array[0];
        assert_eq!(hook.get("timeout").expect("timeout should exist"), 300);
        assert_eq!(hook.get("async").expect("async should exist"), true);
    }

    #[test]
    fn test_remove_hook_no_match() {
        let settings = json!({
            "hooks": [
                {
                    "event": "Start",
                    "command": "/path/to/start.sh",
                    "type": "command",
                    "matcher": ""
                }
            ]
        });

        let result = remove_hook(settings, HookEvent::Stop, "/path/to/stop.sh");
        let hooks = result.get("hooks").expect("hooks array should exist");
        let hooks_array = hooks.as_array().expect("hooks should be array");

        // Should preserve all hooks if no match
        assert_eq!(hooks_array.len(), 1);
        assert_eq!(
            hooks_array[0].get("event").expect("event should exist"),
            "Start"
        );
    }

    #[test]
    #[serial(home)]
    fn test_read_valid_settings() {
        use std::io::Write;
        use tempfile::tempdir;

        // Create temp directory and settings file
        let dir = tempdir().expect("tempdir creation failed");

        // Override HOME for this test
        std::env::set_var("HOME", dir.path());

        // Create .claude directory
        let claude_dir = dir.path().join(".claude");
        fs::create_dir(&claude_dir).expect("mkdir failed");

        // Write valid settings.json
        let settings = json!({
            "hooks": [],
            "cleanupPeriodDays": 7
        });
        let settings_file = claude_dir.join("settings.json");
        let mut file = fs::File::create(&settings_file).expect("file creation failed");
        file.write_all(
            serde_json::to_string_pretty(&settings)
                .expect("serialization failed")
                .as_bytes(),
        )
        .expect("write failed");

        // Test read_settings
        let result = read_settings().expect("read_settings failed");
        assert_eq!(
            result.get("cleanupPeriodDays").expect("should exist"),
            7
        );
        assert!(result.get("hooks").is_some());
    }

    #[test]
    fn test_timestamp_format() {
        use regex::Regex;

        // Create a temp file to verify timestamp format
        let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();

        // Verify format matches yyyyMMdd-hhmmss
        let re = Regex::new(r"^\d{8}-\d{6}$").expect("regex creation failed");
        assert!(
            re.is_match(&timestamp),
            "Timestamp should match format yyyyMMdd-hhmmss, got: {}",
            timestamp
        );
    }
}
