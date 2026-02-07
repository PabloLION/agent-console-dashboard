//! Settings file I/O and manipulation
//!
//! This module handles reading and writing Claude's settings.json with atomic
//! safety guarantees. It preserves all non-hook fields while modifying the
//! hooks object.
//!
//! Claude Code hooks format:
//! ```json
//! {
//!   "hooks": {
//!     "EventName": [
//!       { "matcher": "optional", "hooks": [{ "type": "command", "command": "..." }] }
//!     ]
//!   }
//! }
//! ```

use crate::error::{Result, SettingsError};
use crate::types::{HookEvent, HookHandler, MatcherGroup};
use chrono::Local;
use serde_json::{Map, Value};
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

    // Ensure .claude directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(SettingsError::Io)?;
    }

    // Write to temp file
    let json =
        serde_json::to_string_pretty(&value).map_err(|e| SettingsError::Parse(e.to_string()))?;

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

/// Add hook to settings (pure function, no I/O)
///
/// Adds a hook handler to the specified event. Creates the hooks object and
/// event array if they don't exist.
///
/// # Arguments
///
/// * `value` - The settings.json value
/// * `event` - The hook event (e.g., Stop, PreToolUse)
/// * `handler` - The hook handler configuration
/// * `matcher` - Optional matcher regex (e.g., "Bash" for PreToolUse)
pub fn add_hook(
    mut value: Value,
    event: HookEvent,
    handler: HookHandler,
    matcher: Option<String>,
) -> Value {
    // Ensure hooks object exists
    let root = value.as_object_mut().expect("settings should be object");
    if !root.contains_key("hooks") {
        root.insert("hooks".to_string(), Value::Object(Map::new()));
    }

    let hooks_obj = root
        .get_mut("hooks")
        .and_then(|h| h.as_object_mut())
        .expect("hooks should be object");

    // Get event name as string
    let event_name = serde_json::to_value(event)
        .expect("event serialization failed")
        .as_str()
        .expect("event should serialize to string")
        .to_string();

    // Ensure event array exists
    if !hooks_obj.contains_key(&event_name) {
        hooks_obj.insert(event_name.clone(), Value::Array(Vec::new()));
    }

    let event_array = hooks_obj
        .get_mut(&event_name)
        .and_then(|e| e.as_array_mut())
        .expect("event should be array");

    // Create matcher group with the handler
    let group = MatcherGroup {
        matcher,
        hooks: vec![handler],
    };

    let group_value = serde_json::to_value(group).expect("group serialization failed");
    event_array.push(group_value);

    value
}

/// Remove hook from settings by exact match (pure function, no I/O)
///
/// Removes hooks that match the event and command. If the event array becomes
/// empty, it's preserved (not removed) for consistency.
///
/// # Arguments
///
/// * `value` - The settings.json value
/// * `event` - The hook event to match
/// * `command` - The command string to match
pub fn remove_hook(mut value: Value, event: HookEvent, command: &str) -> Value {
    let hooks_obj = match value.get_mut("hooks").and_then(|h| h.as_object_mut()) {
        Some(obj) => obj,
        None => return value, // No hooks object, nothing to remove
    };

    // Get event name as string
    let event_name = serde_json::to_value(event)
        .expect("event serialization failed")
        .as_str()
        .expect("event should serialize to string")
        .to_string();

    let event_array = match hooks_obj
        .get_mut(&event_name)
        .and_then(|e| e.as_array_mut())
    {
        Some(arr) => arr,
        None => return value, // Event not found, nothing to remove
    };

    // Remove matcher groups that contain the matching command
    event_array.retain(|group| {
        let hooks = group.get("hooks").and_then(|h| h.as_array());
        match hooks {
            Some(hooks_arr) => {
                // Keep if no hook matches the command
                !hooks_arr
                    .iter()
                    .any(|h| h.get("command").and_then(|c| c.as_str()) == Some(command))
            }
            None => true, // Keep malformed entries
        }
    });

    value
}

/// List all hooks from settings (pure function, no I/O)
///
/// Returns a list of (event, matcher, handler) tuples for all hooks in settings.
pub fn list_hooks(value: &Value) -> Vec<(HookEvent, Option<String>, HookHandler)> {
    let mut result = Vec::new();

    let hooks_obj = match value.get("hooks").and_then(|h| h.as_object()) {
        Some(obj) => obj,
        None => return result,
    };

    for (event_name, event_array) in hooks_obj {
        // Parse event name
        let event: HookEvent = match serde_json::from_value(Value::String(event_name.clone())) {
            Ok(e) => e,
            Err(_) => continue, // Skip unknown events
        };

        let groups = match event_array.as_array() {
            Some(arr) => arr,
            None => continue,
        };

        for group in groups {
            let matcher = group
                .get("matcher")
                .and_then(|m| m.as_str())
                .map(String::from);

            let hooks = match group.get("hooks").and_then(|h| h.as_array()) {
                Some(arr) => arr,
                None => continue,
            };

            for hook in hooks {
                let handler: HookHandler = match serde_json::from_value(hook.clone()) {
                    Ok(h) => h,
                    Err(_) => continue,
                };
                result.push((event, matcher.clone(), handler));
            }
        }
    }

    result
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
    fn test_add_hook_creates_structure() {
        let settings = json!({
            "cleanupPeriodDays": 7
        });

        let handler = HookHandler {
            r#type: "command".to_string(),
            command: "/path/to/stop.sh".to_string(),
            timeout: Some(600),
            r#async: None,
            status_message: None,
        };

        let result = add_hook(settings, HookEvent::Stop, handler, None);

        // Check hooks object was created
        let hooks = result.get("hooks").expect("hooks should exist");
        assert!(hooks.is_object(), "hooks should be object");

        // Check Stop array was created
        let stop_array = hooks.get("Stop").expect("Stop should exist");
        assert!(stop_array.is_array(), "Stop should be array");

        // Check matcher group structure
        let groups = stop_array.as_array().expect("should be array");
        assert_eq!(groups.len(), 1);

        let group = &groups[0];
        assert!(group.get("matcher").is_none(), "matcher should be None");

        let inner_hooks = group.get("hooks").expect("hooks should exist");
        let inner_arr = inner_hooks.as_array().expect("should be array");
        assert_eq!(inner_arr.len(), 1);
        assert_eq!(
            inner_arr[0].get("command").expect("cmd").as_str(),
            Some("/path/to/stop.sh")
        );
    }

    #[test]
    fn test_add_hook_with_matcher() {
        let settings = json!({
            "hooks": {}
        });

        let handler = HookHandler {
            r#type: "command".to_string(),
            command: "/path/to/pre-bash.sh".to_string(),
            timeout: Some(10),
            r#async: None,
            status_message: None,
        };

        let result = add_hook(
            settings,
            HookEvent::PreToolUse,
            handler,
            Some("Bash".to_string()),
        );

        let hooks = result.get("hooks").expect("hooks");
        let pre_tool_use = hooks.get("PreToolUse").expect("PreToolUse");
        let groups = pre_tool_use.as_array().expect("array");

        assert_eq!(groups.len(), 1);
        assert_eq!(
            groups[0].get("matcher").expect("matcher").as_str(),
            Some("Bash")
        );
    }

    #[test]
    fn test_add_hook_to_existing_event() {
        let settings = json!({
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "type": "command", "command": "/existing/hook.sh" }
                        ]
                    }
                ]
            }
        });

        let handler = HookHandler {
            r#type: "command".to_string(),
            command: "/new/hook.sh".to_string(),
            timeout: None,
            r#async: None,
            status_message: None,
        };

        let result = add_hook(settings, HookEvent::Stop, handler, None);

        let stop_array = result
            .get("hooks")
            .unwrap()
            .get("Stop")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(stop_array.len(), 2, "should have 2 matcher groups");
    }

    #[test]
    fn test_remove_hook_exact_match() {
        let settings = json!({
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "type": "command", "command": "/path/to/stop.sh" }
                        ]
                    }
                ],
                "SessionStart": [
                    {
                        "hooks": [
                            { "type": "command", "command": "/path/to/start.sh" }
                        ]
                    }
                ]
            }
        });

        let result = remove_hook(settings, HookEvent::Stop, "/path/to/stop.sh");

        let stop_array = result
            .get("hooks")
            .unwrap()
            .get("Stop")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(stop_array.len(), 0, "Stop array should be empty");

        // SessionStart should be preserved
        let start_array = result
            .get("hooks")
            .unwrap()
            .get("SessionStart")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(start_array.len(), 1, "SessionStart should be preserved");
    }

    #[test]
    fn test_remove_hook_preserves_other_groups() {
        let settings = json!({
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "type": "command", "command": "/path/to/stop.sh" }
                        ]
                    },
                    {
                        "hooks": [
                            { "type": "command", "command": "/different/hook.sh" }
                        ]
                    }
                ]
            }
        });

        let result = remove_hook(settings, HookEvent::Stop, "/path/to/stop.sh");

        let stop_array = result
            .get("hooks")
            .unwrap()
            .get("Stop")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(stop_array.len(), 1, "should have 1 remaining group");

        let remaining = &stop_array[0].get("hooks").unwrap().as_array().unwrap()[0];
        assert_eq!(
            remaining.get("command").unwrap().as_str(),
            Some("/different/hook.sh")
        );
    }

    #[test]
    fn test_remove_hook_no_hooks_object() {
        let settings = json!({
            "cleanupPeriodDays": 7
        });

        let result = remove_hook(settings.clone(), HookEvent::Stop, "/any/path");
        assert_eq!(result, settings, "should return unchanged if no hooks");
    }

    #[test]
    fn test_list_hooks_empty() {
        let settings = json!({
            "hooks": {}
        });

        let result = list_hooks(&settings);
        assert!(result.is_empty());
    }

    #[test]
    fn test_list_hooks_multiple() {
        let settings = json!({
            "hooks": {
                "Stop": [
                    {
                        "hooks": [
                            { "type": "command", "command": "/stop.sh", "timeout": 15 }
                        ]
                    }
                ],
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "/pre-bash.sh" }
                        ]
                    }
                ]
            }
        });

        let result = list_hooks(&settings);
        assert_eq!(result.len(), 2);

        // Find Stop hook
        let stop = result.iter().find(|(e, _, _)| *e == HookEvent::Stop);
        assert!(stop.is_some());
        let (_, matcher, handler) = stop.unwrap();
        assert!(matcher.is_none());
        assert_eq!(handler.command, "/stop.sh");
        assert_eq!(handler.timeout, Some(15));

        // Find PreToolUse hook
        let pre = result.iter().find(|(e, _, _)| *e == HookEvent::PreToolUse);
        assert!(pre.is_some());
        let (_, matcher, handler) = pre.unwrap();
        assert_eq!(matcher.as_deref(), Some("Bash"));
        assert_eq!(handler.command, "/pre-bash.sh");
    }

    #[test]
    fn test_roundtrip_preserves_non_hook_keys() {
        let settings = json!({
            "hooks": {},
            "cleanupPeriodDays": 7,
            "env": {"TEST": "value"},
            "permissions": {},
            "statusLine": true,
            "enabledPlugins": ["plugin1"],
            "syntaxHighlightingDisabled": false
        });

        let handler = HookHandler {
            r#type: "command".to_string(),
            command: "/test.sh".to_string(),
            timeout: None,
            r#async: None,
            status_message: None,
        };

        let result = add_hook(settings, HookEvent::Stop, handler, None);

        assert_eq!(result.get("cleanupPeriodDays").expect("should exist"), 7);
        assert!(result.get("env").is_some());
        assert!(result.get("permissions").is_some());
        assert!(result.get("statusLine").is_some());
        assert!(result.get("enabledPlugins").is_some());
        assert!(result.get("syntaxHighlightingDisabled").is_some());
    }

    #[test]
    #[serial(home)]
    fn test_read_valid_settings() {
        use std::io::Write;
        use tempfile::tempdir;

        let dir = tempdir().expect("tempdir creation failed");
        std::env::set_var("HOME", dir.path());

        let claude_dir = dir.path().join(".claude");
        fs::create_dir(&claude_dir).expect("mkdir failed");

        let settings = json!({
            "hooks": {
                "Stop": [{ "hooks": [{ "type": "command", "command": "/test.sh" }] }]
            },
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

        let result = read_settings().expect("read_settings failed");
        assert_eq!(result.get("cleanupPeriodDays").expect("should exist"), 7);
        assert!(result.get("hooks").is_some());
    }

    #[test]
    fn test_timestamp_format() {
        use regex::Regex;

        let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
        let re = Regex::new(r"^\d{8}-\d{6}$").expect("regex creation failed");
        assert!(
            re.is_match(&timestamp),
            "Timestamp should match format yyyyMMdd-hhmmss, got: {}",
            timestamp
        );
    }
}
