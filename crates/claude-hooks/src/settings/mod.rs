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
mod tests;
